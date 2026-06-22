use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct HeaderProps {
    pub site_title: String,
    pub app_version: String,
    pub toggle_theme: Callback<MouseEvent>,
    pub on_logout: Callback<MouseEvent>,
    pub current_theme: String,
    pub is_authenticated: bool,
    pub is_pin_required: bool,
}

#[function_component(Header)]
pub fn header(props: &HeaderProps) -> Html {
    let locale = use_context::<crate::i18n::LocaleContext>().unwrap();

    let current_theme = &props.current_theme;
    let theme_toggle_icon = match current_theme.as_str() {
        "dark" => html! {
            <svg id="moon-icon" class="moon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" /></svg>
        },
        "nord" => html! {
            <svg id="droplet-icon" class="droplet" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22a7 7 0 0 0 7-7c0-4.3-7-13-7-13S5 10.7 5 15a7 7 0 0 0 7 7z"/></svg>
        },
        "dracula" => html! {
            <svg id="sparkles-icon" class="sparkles" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275Z"/><path d="m5 3 1 2.5L8.5 6 6 7 5 9.5 4 7 1.5 6 4 5Z"/><path d="m19 17 1 2.5 2.5.5-2.5 1-1 2.5-1-2.5-2.5-1 2.5-1Z"/></svg>
        },
        "sepia" => html! {
            <svg id="coffee-icon" class="coffee" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 8h1a4 4 0 1 1 0 8h-1"/><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"/><line x1="6" y1="2" x2="6" y2="4"/><line x1="10" y1="2" x2="10" y2="4"/><line x1="14" y1="2" x2="14" y2="4"/></svg>
        },
        _ => html! {
            <svg id="sun-icon" class="sun" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2" /><path d="M12 20v2" /><path d="M4.93 4.93l1.41 1.41" /><path d="M17.66 17.66l1.41 1.41" /><path d="M2 12h2" /><path d="M20 12h2" /><path d="M6.34 17.66l-1.41 1.41" /><path d="M19.07 4.93l-1.41 1.41" /></svg>
        },
    };

    let on_lang_change = {
        let locale = locale.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            locale.on_change.emit(select.value());
        })
    };

    let langs = [
        ("en", "English"),
        ("zh", "简体中文"),
        ("es", "Español"),
        ("de", "Deutsch"),
        ("ja", "日本語"),
        ("fr", "Français"),
        ("pt", "Português"),
        ("ru", "Русский"),
    ];

    html! {
        <header>
            <div id="header-title">
                <h1>{&props.site_title}</h1>
            </div>
            <div class="header-right">
                <div class="language-select-container">
                    <select
                        class="language-select"
                        id="language-select"
                        value={locale.current.clone()}
                        onchange={on_lang_change}
                        aria-label="Select language"
                    >
                        {for langs.iter().map(|&(code, label)| {
                            html! {
                                <option value={code} selected={locale.current == code}>
                                    {label}
                                </option>
                            }
                        })}
                    </select>
                </div>
                <button id="theme-toggle" class="icon-button" onclick={props.toggle_theme.clone()} aria-label="Toggle theme">
                    {theme_toggle_icon}
                </button>
                <button
                    id="logout-button"
                    class="icon-button"
                    onclick={props.on_logout.clone()}
                    disabled={!props.is_authenticated || !props.is_pin_required}
                    data-tooltip={if !props.is_authenticated || !props.is_pin_required { "".to_string() } else { locale.t("logout") }}
                >
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><polyline points="16 17 21 12 16 7" /><line x1="21" y1="12" x2="9" y2="12" /></svg>
                </button>
            </div>
        </header>
    }
}
