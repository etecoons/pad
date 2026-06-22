use yew::prelude::*;

mod de;
mod en;
mod es;
mod fr;
mod ja;
mod pt;
mod ru;
mod zh;

#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    pub current: String,
    pub on_change: Callback<String>,
}

impl LocaleContext {
    pub fn t(&self, key: &str) -> String {
        translate(&self.current, key)
    }
}

pub fn detect_browser_locale() -> String {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        if let Some(lang) = navigator.language() {
            let l = lang.to_lowercase();
            if l.starts_with("zh") { return "zh".to_string(); }
            if l.starts_with("es") { return "es".to_string(); }
            if l.starts_with("de") { return "de".to_string(); }
            if l.starts_with("ja") { return "ja".to_string(); }
            if l.starts_with("fr") { return "fr".to_string(); }
            if l.starts_with("pt") { return "pt".to_string(); }
            if l.starts_with("ru") { return "ru".to_string(); }
        }
    }
    "en".to_string()
}

pub fn get_saved_locale() -> String {
    crate::storage::StorageService::get_item("lang", &detect_browser_locale())
}

pub fn set_saved_locale(locale: &str) {
    crate::storage::StorageService::set_item("lang", locale);
}

pub fn translate(lang: &str, key: &str) -> String {
    let l = if lang.starts_with("zh") {
        "zh"
    } else if lang.starts_with("es") {
        "es"
    } else if lang.starts_with("de") {
        "de"
    } else if lang.starts_with("ja") {
        "ja"
    } else if lang.starts_with("fr") {
        "fr"
    } else if lang.starts_with("pt") {
        "pt"
    } else if lang.starts_with("ru") {
        "ru"
    } else {
        "en"
    };

    let val = match l {
        "zh" => zh::translate(key),
        "es" => es::translate(key),
        "de" => de::translate(key),
        "ja" => ja::translate(key),
        "fr" => fr::translate(key),
        "pt" => pt::translate(key),
        "ru" => ru::translate(key),
        _ => en::translate(key),
    };

    val.unwrap_or(key).to_string()
}
