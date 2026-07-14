use crate::backend::api::{get_home_info, get_random_synset};
use crate::components::WordNet;
use crate::Route;
use dioxus::prelude::*;

static CSS: Asset = asset!("/assets/styling/home.css");

/// "142,384" - thousands-grouped, so the stat row is readable at a glance.
fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    grouped.chars().rev().collect()
}

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    let info = use_loader(get_home_info);
    let navigator = use_navigator();

    rsx! {
        document::Style { href: CSS },
        div {
            class: "home",
            div {
                class: "home-search",
                WordNet {},
            }
            match &info {
                Ok(loaded) if !loaded.loading() => {
                    let info = loaded.read();
                    rsx! {
                        h2 { class: "home-tagline", "{info.tagline}" }
                        div {
                            class: "home-intro",
                            dangerous_inner_html: "{info.intro}"
                        }
                        div {
                            class: "home-actions",
                            span { class: "home-stat", "{format_count(info.n_synsets)} synsets" }
                            span { class: "home-stat", "{format_count(info.n_entries)} entries" }
                            button {
                                class: "home-stat home-random",
                                onclick: move |_| async move {
                                    if let Ok(Some(id)) = get_random_synset().await {
                                        navigator.push(Route::BySynset { synset: id.as_str().to_string() });
                                    }
                                },
                                "Random synset ›"
                            }
                        }
                    }
                }
                _ => rsx! {},
            }
        }
    }
}
