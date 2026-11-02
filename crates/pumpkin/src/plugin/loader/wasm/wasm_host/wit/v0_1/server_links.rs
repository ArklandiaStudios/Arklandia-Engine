use crate::plugin::loader::wasm::wasm_host::{state::PluginHostState, wit::v0_1::pumpkin::plugin::server_links::Host};

impl PluginHostState {
    fn link_list(&self) -> Vec<(String, String)> {
        let Some(server) = self.server.as_ref() else {
            return Vec::new();
        };
        let cfg = &server.advanced_config.server_links;
        let mut out = Vec::new();
        for (name, url) in [
            ("bug", cfg.bug_report.as_str()),
            ("discord", cfg.discord.as_str()),
            ("site", cfg.website.as_str()),
        ] {
            if !url.trim().is_empty() {
                out.push((name.to_string(), url.trim().to_string()));
            }
        }
        let mut custom: Vec<_> = cfg.custom.iter().filter(|(_, v)| !v.trim().is_empty()).collect();
        custom.sort();
        out.extend(custom.into_iter().map(|(k, v)| (k.clone(), v.trim().to_string())));
        out
    }
}

impl Host for PluginHostState {
    async fn server_links(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        Ok(self.link_list())
    }

    async fn get_server_link(&mut self, name: String) -> wasmtime::Result<Option<String>> {
        Ok(self
            .link_list()
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name))
            .map(|(_, u)| u))
    }
}
