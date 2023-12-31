use std::collections::BTreeMap;

use once_cell::sync::Lazy;

pub static CONFIG_PATH: &str = "config.toml";
pub static SOURCE_CODE_SAVED_PATH: &str = "assets/src";

pub static LANG_EXTENSIONS: Lazy<BTreeMap<String, String>> = Lazy::new(|| {
    BTreeMap::from([
        ("c".to_string(), "c".to_string()),
        ("cpp".to_string(), "cpp".to_string()),
        ("rust".to_string(), "rs".to_string()),
        ("python".to_string(), "py".to_string()),
    ])
});
