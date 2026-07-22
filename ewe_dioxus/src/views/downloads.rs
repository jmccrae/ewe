#![cfg(not(feature = "desktop"))]
use dioxus::prelude::*;
use crate::backend::downloads::get_downloads;
use crate::components::{ProjectName, WordNet};

/// "1.2 MB", "512 KB", "43 B" - whichever unit keeps the number readable.
fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

#[component]
pub fn Downloads() -> Element {
    let releases = use_loader(get_downloads);
    let project_name = use_context::<Signal<ProjectName>>();

    rsx! {
        div {
            if !project_name().0.is_empty() {
                document::Title { "Downloads - {project_name().0}" }
            }
            WordNet {},
            div {
                class: "downloads",
                h2 { "Downloads" }
                match &releases {
                    Ok(loaded) if !loaded.loading() => {
                        let releases = loaded.read();
                        if releases.is_empty() {
                            rsx! {
                                p { "No downloads are available." }
                            }
                        } else {
                            rsx! {
                                for release in releases.iter() {
                                    div {
                                        key: "{release.version}",
                                        class: "downloads-release",
                                        h3 {
                                            "{release.version}"
                                            if let Some(date) = &release.date {
                                                span { class: "downloads-date", " ({date})" }
                                            }
                                        }
                                        if let Some(description) = &release.description {
                                            p { class: "downloads-description", "{description}" }
                                        }
                                        table {
                                            class: "downloads-files",
                                            tbody {
                                                for file in release.files.iter() {
                                                    {
                                                        let format = file.format.clone().unwrap_or_default();
                                                        let description = file.description.clone().unwrap_or_default();
                                                        let size = file.size_bytes.map(format_size).unwrap_or_else(|| "unavailable".to_string());
                                                        rsx! {
                                                            tr {
                                                                key: "{file.filename}",
                                                                td {
                                                                    class: "downloads-filename",
                                                                    a { href: "/downloads/{file.filename}", "{file.filename}" }
                                                                }
                                                                td { class: "downloads-format", "{format}" }
                                                                td { class: "downloads-file-description", "{description}" }
                                                                td { class: "downloads-size", "{size}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) => rsx! { p { "Loading..." } },
                    Err(_) => rsx! { p { "Failed to load downloads." } },
                }
            }
        }
    }
}
