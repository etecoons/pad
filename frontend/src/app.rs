use crate::editor::Editor;
use crate::header::Header;
use crate::login::Login;
use crate::services::{ApiService, StorageService};
use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let authenticated = use_state(|| false);
    let app_version = use_state(|| "1.0.6".to_string());
    let site_title = use_state(|| "RustPad".to_string());
    let theme = use_state(StorageService::get_theme);
    let locale_state = use_state(crate::i18n::get_saved_locale);
    let active_notification = use_state(|| None::<(String, String)>);

    {
        let version = app_version.clone();
        let site_title = site_title.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(config) = ApiService::get_config().await {
                    version.set(config.version);
                    site_title.set(config.site_title.clone());
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            doc.set_title(&config.site_title);
                        }
                    }
                }
            });
            || ()
        });
    }

    {
        let authenticated = authenticated.clone();

        use_effect_with(*authenticated, move |&auth| {
            if auth {
                spawn_local(async move {
                    // Fetch default notes to make sure default notepad is initialized
                    let _ = ApiService::get_notes("default").await;
                });
            }
            || ()
        });
    }

    let locale_on_change = {
        let ls = locale_state.clone();
        Callback::from(move |new_lang: String| {
            crate::i18n::set_saved_locale(&new_lang);
            ls.set(new_lang);
        })
    };
    let locale_context = crate::i18n::LocaleContext {
        current: (*locale_state).clone(),
        on_change: locale_on_change,
    };

    let toggle_theme = {
        let theme = theme.clone();
        Callback::from(move |_| {
            let next = match theme.as_str() {
                "light" => "dark",
                "dark" => "nord",
                "nord" => "dracula",
                "dracula" => "sepia",
                _ => "light",
            };
            StorageService::set_theme(next);
            let _ = window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
                .map(|r| r.set_attribute("data-theme", next));
            theme.set(next.to_string());
        })
    };

    let on_logout = {
        let auth = authenticated.clone();
        Callback::from(move |_| {
            let auth = auth.clone();
            spawn_local(async move {
                if ApiService::logout().await.is_ok() {
                    auth.set(false);
                }
            });
        })
    };

    let current_theme = StorageService::get_theme();
    let theme_stylesheet_url = if current_theme == "dark" {
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css"
    } else {
        "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css"
    };

    let ver_val = (*app_version).clone();

    html! {
        <ContextProvider<crate::i18n::LocaleContext> context={locale_context}>
            <Header
                site_title={(*site_title).clone()}
                app_version={ver_val}
                toggle_theme={toggle_theme}
                on_logout={on_logout}
                current_theme={(*theme).clone()}
                is_authenticated={*authenticated}
            />
            <div class="container">
                <link rel="stylesheet" href={theme_stylesheet_url} />
                {if !*authenticated {
                    html! { <Login on_login_success={let auth = authenticated.clone(); Callback::from(move |_| auth.set(true))} /> }
                } else {
                    html! {
                        <main>
                            <Editor
                                notepad_id={"default".to_string()}
                                save_interval={3000}
                                disable_print_expand={false}
                                on_status={let active_notif = active_notification.clone(); Callback::from(move |status| active_notif.set(status))}
                            />
                        </main>
                    }
                }}
            </div>
            <footer class="layout-footer">
                {
                    if let Some((msg, cls)) = &*active_notification {
                        html! { <div class={format!("footer-status-text {}", cls)}>{ msg }</div> }
                    } else {
                        html! {}
                    }
                }
            </footer>
        </ContextProvider<crate::i18n::LocaleContext>>
    }
}
