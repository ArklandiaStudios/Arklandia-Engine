//! Automatic completion of `pumpkin.toml` when a new version adds settings.
//!
//! * missing keys are added with their default value (deep, per section); values already in
//!   the file are never changed, arrays are kept whole, unknown keys are kept;
//! * the file is edited **as text**: the user's comments, order and layout are preserved, new
//!   keys are appended at the end of their section (or in a new section at the end of the file);
//! * the file is rewritten only when something was added, atomically (temporary file then
//!   rename), after a backup `pumpkin.toml.bak-YYYYMMDD-HHMMSS` (the 3 latest are kept);
//! * a file with a newer `config_version` than this program is never touched.

use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

/// Version of the configuration layout understood by this program.
pub const CONFIG_VERSION: u32 = 1;

/// Version stored in a parsed file (0 when absent).
#[must_use]
pub fn file_version(value: &Value) -> u32 {
    value
        .get("config_version")
        .and_then(Value::as_integer)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

/// Deep merge: defaults, overlaid by the user's values (arrays and scalars are the user's).
#[must_use]
pub fn deep_merge(default: &Value, user: &Value) -> Value {
    match (default, user) {
        (Value::Table(d), Value::Table(u)) => {
            let mut out = Table::new();
            for (k, dv) in d {
                match u.get(k) {
                    Some(uv) => out.insert(k.clone(), deep_merge(dv, uv)),
                    None => out.insert(k.clone(), dv.clone()),
                };
            }
            for (k, uv) in u {
                if !d.contains_key(k) {
                    out.insert(k.clone(), uv.clone());
                }
            }
            Value::Table(out)
        }
        (_, u) => u.clone(),
    }
}

fn leaves(v: &Value) -> usize {
    match v {
        Value::Table(t) if !t.is_empty() => t.values().map(leaves).sum(),
        _ => 1,
    }
}

/// Dotted paths of the keys present in `user` but unknown to `default` (empty default tables,
/// such as `commands.overrides`, are free-form and skipped).
#[must_use]
pub fn unknown_keys(default: &Value, user: &Value) -> Vec<String> {
    fn walk(prefix: &str, d: &Table, u: &Table, out: &mut Vec<String>) {
        for (k, uv) in u {
            let path = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
            match d.get(k) {
                None => out.push(path),
                Some(Value::Table(dt)) if !dt.is_empty() => {
                    if let Value::Table(ut) = uv {
                        walk(&path, dt, ut, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let (Value::Table(d), Value::Table(u)) = (default, user) {
        walk("", d, u, &mut out);
    }
    out
}

/// Missing entries grouped by existing section.
struct Plan {
    /// (section path, scalar/array entries to add inside it)
    inline: Vec<(Vec<String>, Vec<(String, Value)>)>,
    /// whole tables to append at the end: (path, table)
    tables: Vec<(Vec<String>, Value)>,
    /// every added key, for the log (top-most missing keys)
    added: Vec<String>,
    /// number of added values
    count: usize,
}

fn is_table_like(v: &Value) -> bool {
    match v {
        Value::Table(_) => true,
        Value::Array(a) => !a.is_empty() && a.iter().all(Value::is_table),
        _ => false,
    }
}

fn plan(default: &Table, user: &Table, path: &mut Vec<String>, p: &mut Plan) {
    let mut entries = Vec::new();
    for (k, dv) in default {
        match user.get(k) {
            None => {
                let full = path.iter().chain(std::iter::once(k)).cloned().collect::<Vec<_>>();
                p.added.push(full.join("."));
                p.count += leaves(dv);
                if is_table_like(dv) {
                    p.tables.push((full, dv.clone()));
                } else {
                    entries.push((k.clone(), dv.clone()));
                }
            }
            Some(Value::Table(ut)) => {
                if let Value::Table(dt) = dv {
                    path.push(k.clone());
                    plan(dt, ut, path, p);
                    path.pop();
                }
            }
            Some(_) => {}
        }
    }
    if !entries.is_empty() {
        p.inline.push((path.clone(), entries));
    }
}

/// Splits a table header `a.b."c d"` into its parts.
fn header_path(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with('[') || t.starts_with("[[") {
        return None;
    }
    let end = t.rfind(']')?;
    let inner = &t[1..end];
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in inner.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => quote = Some(c),
            (None, '.') => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            (None, c) => cur.push(c),
        }
    }
    parts.push(cur.trim().to_string());
    Some(parts)
}

fn is_header(line: &str) -> bool {
    line.trim_start().starts_with('[')
}

fn render(path: &[String], v: &Value) -> String {
    // Wrap `v` in nested tables so the serializer writes the right [a.b] headers.
    let mut wrapped = v.clone();
    for k in path.iter().rev() {
        let mut t = Table::new();
        t.insert(k.clone(), wrapped);
        wrapped = Value::Table(t);
    }
    toml::to_string(&wrapped).unwrap_or_default()
}

const NOTE: &str = "# Ajouté automatiquement par la mise à jour de la configuration (valeurs par défaut).";

/// Adds the missing default values to `text` (the content of the file) without touching
/// anything else. Returns `(new_text, added_keys, added_value_count)`. The result is checked:
/// if it does not parse back to exactly `deep_merge(default, user)`, `None` is returned and the
/// caller must keep the file as is.
#[must_use]
pub fn insert_missing(text: &str, default: &Value, user: &Value) -> Option<(String, Vec<String>, usize)> {
    let (Value::Table(d), Value::Table(u)) = (default, user) else {
        return None;
    };
    let mut p = Plan { inline: vec![], tables: vec![], added: vec![], count: 0 };
    plan(d, u, &mut Vec::new(), &mut p);
    if p.count == 0 {
        return Some((text.to_string(), vec![], 0));
    }
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut tail = String::new();
    // (insertion index, text block)
    let mut inserts: Vec<(usize, String)> = Vec::new();
    for (path, entries) in &p.inline {
        let mut table = Table::new();
        for (k, v) in entries {
            table.insert(k.clone(), v.clone());
        }
        let body = toml::to_string(&Value::Table(table)).ok()?;
        let headers: Vec<(usize, Vec<String>)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| header_path(l).map(|h| (i, h)))
            .collect();
        let start = if path.is_empty() { Some(0) } else { headers.iter().find(|(_, h)| h == path).map(|(i, _)| i + 1) };
        match start {
            Some(start) => {
                let end = lines
                    .iter()
                    .enumerate()
                    .skip(start)
                    .find(|(_, l)| is_header(l))
                    .map_or(lines.len(), |(i, _)| i);
                let mut at = start;
                for (i, l) in lines.iter().enumerate().take(end).skip(start) {
                    let t = l.trim();
                    if !t.is_empty() && !t.starts_with('#') {
                        at = i + 1;
                    }
                }
                inserts.push((at, format!("{NOTE}\n{body}")));
            }
            None => {
                // The section only exists implicitly (e.g. through [a.b.c]): create it at the end.
                let mut t = Table::new();
                for (k, v) in entries {
                    t.insert(k.clone(), v.clone());
                }
                tail.push_str(&format!("\n{NOTE}\n{}", render(path, &Value::Table(t))));
            }
        }
    }
    for (path, v) in &p.tables {
        tail.push_str(&format!("\n{NOTE}\n{}", render(path, v)));
    }
    inserts.sort_by(|a, b| b.0.cmp(&a.0));
    for (at, block) in inserts {
        let block_lines: Vec<String> = block.trim_end().lines().map(String::from).collect();
        lines.splice(at..at, block_lines);
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&tail);
    // Safety net: the new file must mean exactly "defaults overlaid by the user's values".
    let reparsed: Value = toml::from_str(&out).ok()?;
    if reparsed != deep_merge(default, user) {
        return None;
    }
    Some((out, p.added, p.count))
}

fn stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86400).unwrap_or(0);
    let rem = secs % 86400;
    // civil from days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Backs the file up, then replaces it atomically with `new_text`. Returns the backup path.
///
/// # Errors
/// Any I/O error (read-only file system...): the caller keeps the merge in memory only.
pub fn backup_and_write(path: &Path, new_text: &str) -> std::io::Result<PathBuf> {
    let name = path.file_name().map_or_else(|| "config".into(), |n| n.to_string_lossy().into_owned());
    let dir = path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let backup = dir.join(format!("{name}.bak-{}", stamp()));
    fs::copy(path, &backup)?;
    let tmp = dir.join(format!("{name}.tmp"));
    fs::write(&tmp, new_text)?;
    fs::rename(&tmp, path)?;
    // keep the 3 latest backups
    let prefix = format!("{name}.bak-");
    let mut all: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&prefix)))
        .collect();
    all.sort();
    while all.len() > 3 {
        let _ = fs::remove_file(all.remove(0));
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Value {
        toml::from_str(s).unwrap()
    }

    const DEFAULT: &str = r#"
config_version = 1
seed = "1"
[chat]
format = "<x>"
new_key = 5
[messages]
join = "a"
[messages.deep]
x = 1
y = [1, 2]
[commands]
list = ["a", "b"]
[commands.overrides]
"#;

    #[test]
    fn adds_missing_keeps_user_comments_and_unknown() {
        let user_text = "# mon serveur\nseed = \"42\" # ma graine\nmystery = true\n\n# chat\n[chat]\nformat = \"<me>\"\n\n[commands]\nlist = [\"z\"]\n\n[commands.overrides.kick]\nenabled = false\n";
        let (out, added, count) = insert_missing(user_text, &v(DEFAULT), &v(user_text)).unwrap();
        assert!(out.contains("# mon serveur") && out.contains("# ma graine") && out.contains("# chat"));
        assert!(out.contains("seed = \"42\"") && out.contains("format = \"<me>\"") && out.contains("mystery = true"));
        assert!(out.contains("list = [\"z\"]"), "arrays are not merged");
        let merged = v(&out);
        assert_eq!(merged["config_version"].as_integer(), Some(1));
        assert_eq!(merged["chat"]["new_key"].as_integer(), Some(5));
        assert_eq!(merged["messages"]["deep"]["y"].as_array().unwrap().len(), 2);
        assert_eq!(merged["commands"]["overrides"]["kick"]["enabled"].as_bool(), Some(false));
        assert!(added.contains(&"messages".to_string()) && added.contains(&"chat.new_key".to_string()));
        assert_eq!(count, 1 + 1 + 3);
        // chat.new_key lands inside [chat], before [commands]
        assert!(out.find("new_key").unwrap() < out.find("[commands]").unwrap());
        // second pass: nothing to add, text identical
        let (again, a2, c2) = insert_missing(&out, &v(DEFAULT), &v(&out)).unwrap();
        assert!(a2.is_empty() && c2 == 0 && again == out);
    }

    #[test]
    fn implicit_section_and_unknown_listing() {
        let user = "[messages.deep]\nx = 9\n";
        let (out, _, _) = insert_missing(user, &v(DEFAULT), &v(user)).unwrap();
        let m = v(&out);
        assert_eq!(m["messages"]["deep"]["x"].as_integer(), Some(9));
        assert_eq!(m["messages"]["join"].as_str(), Some("a"));
        assert_eq!(unknown_keys(&v(DEFAULT), &v("zzz = 1\n[chat]\nfoo = 1\n[commands.overrides.kick]\nenabled = false\n")), vec!["chat.foo".to_string(), "zzz".to_string()]);
    }

    #[test]
    fn newer_version_is_detected() {
        assert_eq!(file_version(&v("config_version = 7")), 7);
        assert_eq!(file_version(&v("a = 1")), 0);
    }
}
