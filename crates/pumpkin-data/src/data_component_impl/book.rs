use crate::data_component_impl::DataComponentImpl;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::text::TextComponent;

/// Vanilla books retain both the original content and its optional filtered form.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Filterable<T> {
    pub raw: T,
    pub filtered: Option<T>,
}

impl<T> Filterable<T> {
    pub const fn new(raw: T) -> Self {
        Self { raw, filtered: None }
    }
}

fn read_filterable<T>(tag: &NbtTag, read: impl Fn(&NbtTag) -> Option<T>) -> Option<Filterable<T>> {
    if let NbtTag::Compound(compound) = tag
        && let Some(raw) = compound.get("raw")
    {
        return Some(Filterable {
            raw: read(raw)?,
            filtered: compound.get("filtered").and_then(&read),
        });
    }
    Some(Filterable::new(read(tag)?))
}

fn write_filterable<T>(value: &Filterable<T>, write: impl Fn(&T) -> NbtTag) -> NbtTag {
    let mut compound = NbtCompound::new();
    compound.put("raw", write(&value.raw));
    if let Some(filtered) = &value.filtered {
        compound.put("filtered", write(filtered));
    }
    NbtTag::Compound(compound)
}

fn read_string(tag: &NbtTag) -> Option<String> {
    tag.extract_string().map(ToString::to_string)
}

fn write_string(value: &String) -> NbtTag {
    NbtTag::String(value.clone().into_boxed_str())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct WritableBookContentImpl {
    pub pages: Vec<Filterable<String>>,
}

impl WritableBookContentImpl {
    pub fn read_data(tag: &NbtTag) -> Option<Self> {
        let mut pages = Vec::new();
        if let NbtTag::Compound(compound) = tag
            && let Some(NbtTag::List(values)) = compound.get("pages")
        {
            pages.extend(values.iter().filter_map(|v| read_filterable(v, read_string)));
        }
        Some(Self { pages })
    }
}

impl DataComponentImpl for WritableBookContentImpl {
    fn write_data(&self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.put("pages", NbtTag::List(self.pages.iter().map(|p| write_filterable(p, write_string)).collect()));
        NbtTag::Compound(compound)
    }
    default_impl!(WritableBookContent);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct WrittenBookContentImpl {
    pub title: Filterable<String>,
    pub author: String,
    pub pages: Vec<Filterable<TextComponent>>,
    pub generation: i32,
    pub resolved: bool,
}

impl WrittenBookContentImpl {
    pub fn read_data(tag: &NbtTag) -> Option<Self> {
        let compound = tag.extract_compound()?;
        let title = compound.get("title")
            .and_then(|v| read_filterable(v, read_string))
            .unwrap_or_else(|| Filterable::new(String::new()));
        let author = compound.get_string("author").map_or_else(String::new, ToString::to_string);
        let mut pages = Vec::new();
        if let Some(NbtTag::List(values)) = compound.get("pages") {
            pages.extend(values.iter().filter_map(|v| read_filterable(v, |text| Some(TextComponent::from_nbt(text)))));
        }
        Some(Self {
            title, author, pages,
            generation: compound.get_int("generation").unwrap_or(0),
            resolved: compound.get_bool("resolved").unwrap_or(false),
        })
    }
}

impl DataComponentImpl for WrittenBookContentImpl {
    fn write_data(&self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.put("title", write_filterable(&self.title, write_string));
        compound.put_string("author", self.author.clone());
        compound.put("pages", NbtTag::List(self.pages.iter().map(|p| write_filterable(p, |text| text.to_nbt_tag_for_version(&pumpkin_util::version::JavaMinecraftVersion::V_26_3))).collect()));
        compound.put_int("generation", self.generation);
        compound.put_bool("resolved", self.resolved);
        NbtTag::Compound(compound)
    }
    default_impl!(WrittenBookContent);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DebugStickStateImpl;
impl DebugStickStateImpl {
    pub const fn read_data(_data: &NbtTag) -> Option<Self> {
        Some(Self)
    }
}
impl DataComponentImpl for DebugStickStateImpl {
    default_impl!(DebugStickState);
}
