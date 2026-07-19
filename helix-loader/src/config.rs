#![allow(clippy::missing_errors_doc)]
use std::str::from_utf8;

/// Default built-in languages.toml.
#[must_use]
pub fn default_lang_config() -> toml::Value {
    let default_config = include_bytes!("../../languages.toml");
    // let default_config = include_bytes!("/home/pk/.config/helix/languages.toml");

    toml::from_str(from_utf8(default_config).unwrap())
        .expect("Could not parse built-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
///
/// Workspace-local `.helix/languages.toml` is merged in only when the current
/// workspace is trusted for [`TrustQuery::LocalConfig`].
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let global_config = crate::lang_config_file();

    let files = [global_config];

    let config = files
        .iter()
        .filter_map(|file| {
            std::fs::read_to_string(file)
                .map(|config| toml::from_str(&config))
                .ok()
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(default_lang_config(), |a, b| {
            crate::merge_toml_values(a, b, 3)
        });

    Ok(config)
}
