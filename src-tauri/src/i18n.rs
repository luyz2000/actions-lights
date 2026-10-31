//! Text in the system language. The catalogs live in `src/lib/i18n/` and are shared with
//! the frontend, which gets the chosen language through the `get_locale` command.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

const CATALOGS: [(&str, &str); 6] = [
    ("en", include_str!("../../src/lib/i18n/en.json")),
    ("es", include_str!("../../src/lib/i18n/es.json")),
    ("ru", include_str!("../../src/lib/i18n/ru.json")),
    ("fr", include_str!("../../src/lib/i18n/fr.json")),
    ("ja", include_str!("../../src/lib/i18n/ja.json")),
    ("zh", include_str!("../../src/lib/i18n/zh.json")),
];

type Catalog = HashMap<String, String>;

fn catalogs() -> &'static HashMap<&'static str, Catalog> {
    static PARSED: OnceLock<HashMap<&'static str, Catalog>> = OnceLock::new();
    PARSED.get_or_init(|| {
        CATALOGS
            .iter()
            .map(|(code, json)| (*code, serde_json::from_str(json).expect("valid catalog")))
            .collect()
    })
}

/// First supported language among the preferred ones ("es-MX", "zh_CN"…), else English.
pub fn resolve<S: AsRef<str>>(preferred: &[S]) -> &'static str {
    preferred
        .iter()
        .filter_map(|tag| {
            let lang = tag.as_ref().split(['-', '_']).next()?.to_ascii_lowercase();
            CATALOGS
                .iter()
                .find(|(code, _)| *code == lang)
                .map(|(code, _)| *code)
        })
        .next()
        .unwrap_or("en")
}

/// The language picked in Settings › General, if supported; otherwise the system's.
pub fn choose<S: AsRef<str>>(chosen: Option<&str>, system: &[S]) -> &'static str {
    chosen
        .and_then(|code| CATALOGS.iter().find(|(c, _)| *c == code))
        .map(|(c, _)| *c)
        .unwrap_or_else(|| resolve(system))
}

/// The user's choice from the config; None follows the system.
static CHOSEN: RwLock<Option<String>> = RwLock::new(None);

pub fn set_choice(code: Option<&str>) {
    *CHOSEN.write().unwrap() = code.map(String::from);
}

/// The language texts are shown in right now.
pub fn locale() -> &'static str {
    // Tests assert the Spanish texts, whatever the machine's language.
    if cfg!(test) {
        return "es";
    }
    static SYSTEM: OnceLock<Vec<String>> = OnceLock::new();
    let system = SYSTEM.get_or_init(|| sys_locale::get_locales().collect());
    choose(CHOSEN.read().unwrap().as_deref(), system)
}

fn lookup(code: &str, key: &str) -> String {
    let all = catalogs();
    all.get(code)
        .and_then(|c| c.get(key))
        .or_else(|| all["en"].get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn t(key: &str) -> String {
    lookup(locale(), key)
}

/// `t` with `{name}` placeholders filled in.
pub fn tf(key: &str, vars: &[(&str, &dyn Display)]) -> String {
    fill(lookup(locale(), key), vars)
}

fn fill(mut text: String, vars: &[(&str, &dyn Display)]) -> String {
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_first_supported_system_language() {
        assert_eq!(resolve(&["es-MX", "en-US"]), "es");
        assert_eq!(resolve(&["de-DE", "fr-CA"]), "fr");
        assert_eq!(resolve(&["zh-Hans-CN"]), "zh");
        assert_eq!(resolve(&["ja_JP"]), "ja");
        assert_eq!(resolve(&["RU"]), "ru");
    }

    #[test]
    fn a_chosen_language_wins_over_the_system() {
        assert_eq!(choose(Some("ja"), &["es-MX"]), "ja");
        assert_eq!(choose(None, &["es-MX"]), "es");
        // A choice from a newer version that this one doesn't know: follow the system.
        assert_eq!(choose(Some("de"), &["fr-FR"]), "fr");
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(resolve(&["de-DE"]), "en");
        assert_eq!(resolve::<&str>(&[]), "en");
        assert_eq!(lookup("xx", "tray.menu.quit"), "Quit");
    }

    #[test]
    fn every_catalog_parses_and_has_the_english_keys() {
        let all = catalogs();
        for (code, _) in CATALOGS {
            for key in all["en"]
                .keys()
                .filter(|k| !k.contains(".one") && !k.contains(".other"))
            {
                assert!(all[code].contains_key(key), "{code} is missing {key}");
            }
        }
    }

    #[test]
    fn fills_placeholders() {
        let text = fill(
            lookup("en", "step.named"),
            &[("n", &2), ("total", &5), ("name", &"Build")],
        );
        assert_eq!(text, "Step 2/5 · Build");
        assert_eq!(
            lookup("ja", "update.available").replace("{version}", "2.0"),
            "バージョン 2.0 が利用可能"
        );
    }
}
