use skald_core::config::ResolvedConfig;
use skald_core::output::OutputFormat;

pub fn run_aliases(
    config: &ResolvedConfig,
    format: OutputFormat,
    is_tty: bool,
    show_source: bool,
) -> i32 {
    if config.aliases.is_empty() {
        cliclack::log::info("No aliases configured.").ok();
        return 0;
    }

    let mut sorted: Vec<(&String, &String)> = config.aliases.iter().collect();
    sorted.sort_by_key(|(name, _)| name.as_str());

    if show_source {
        let headers = vec!["Alias", "Expansion", "Source"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, expansion)| {
                let source = config
                    .sources
                    .get(format!("alias.{name}").as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "config".to_string());
                vec![(*name).clone(), (*expansion).clone(), source]
            })
            .collect();
        print!("{}", format.render_rows(&headers, &rows, is_tty));
    } else {
        let headers = vec!["Alias", "Expansion"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, expansion)| vec![(*name).clone(), (*expansion).clone()])
            .collect();
        print!("{}", format.render_rows(&headers, &rows, is_tty));
    }

    0
}
