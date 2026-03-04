use dioxus::prelude::*;
use phf::phf_map;
use std::collections::HashMap;

static FRAMEMAP: phf::Map<&'static str, &'static str> = phf_map! {
        "nonreferential" => "It is ----ing",
        "nonreferential-sent" => "It ----s that CLAUSE",
        "ditransitive" => "Somebody ----s somebody something",
        "via" => "Somebody ----s",
        "via-adj" => "Somebody ----s Adjective",
        "via-at" => "Somebody ----s at something",
        "via-for" => "Somebody ----s for something",
        "via-ger" => "Somebody ----s VERB-ing",
        "via-inf" => "Somebody ----s INFINITIVE",
        "via-on-anim" => "Somebody ----s on somebody",
        "via-on-inanim" => "Somebody ----s on something",
        "via-out-of" => "Somebody ----s out of somebody",
        "via-pp" => "Somebody ----s PP",
        "via-that" => "Somebody ----s that CLAUSE",
        "via-to" => "Somebody ----s to somebody",
        "via-to-inf" => "Somebody ----s to INFINITIVE",
        "via-whether-inf" => "Somebody ----s whether INFINITIVE",
        "vibody" => "Somebody's (body part) ----s",
        "vii" => "Something ----s",
        "vii-adj" => "Something ----s Adjective/Noun",
        "vii-inf" => "Something ----s INFINITIVE",
        "vii-pp" => "Something is ----ing PP",
        "vii-to" => "Something ----s to somebody",
        "vtaa" => "Somebody ----s somebody",
        "vtaa-inf" => "Somebody ----s somebody INFINITIVE",
        "vtaa-into-ger" => "Somebody ----s somebody into V-ing something",
        "vtaa-of" => "Somebody ----s somebody of something",
        "vtaa-pp" => "Somebody ----s somebody PP",
        "vtaa-to-inf" => "Somebody ----s somebody to INFINITIVE",
        "vtaa-with" => "Somebody ----s somebody with something",
        "vtai" => "Somebody ----s something",
        "vtai-from" => "Somebody ----s something from somebody",
        "vtai-on" => "Somebody ----s something on somebody",
        "vtai-pp" => "Somebody ----s something PP",
        "vtai-to" => "Somebody ----s something to somebody",
        "vtai-with" => "Somebody ----s something with something",
        "vtia" => "Something ----s somebody",
        "vtii" => "Something ----s something",
        "vtii-adj" => "Something ----s something Adjective/Noun"
};

fn third_person_form(word : &str) -> String {
    if word.ends_with('s') {
        format!("{}es", word)
    } else if word.ends_with("ay") || word.ends_with("ey") || word.ends_with("iy") || word.ends_with("oy") || word.ends_with("uy") {
        format!("{}s", word)
    } else if word.ends_with('y') {
        format!("{}ies", &word[..word.len()-1])
    } else if word.ends_with('e') {
        format!("{}s", word)
    } else if word.ends_with('o') {
        format!("{}es", word)
    } else if word.ends_with("ch") {
        format!("{}es", word)
    } else if word.ends_with("sh") {
        format!("{}es", word)
    } else if word.ends_with('x') {
        format!("{}es", word)
    } else {
        format!("{}s", word)
    }
}

fn gerund_form(word : &str) -> String {
    if word.ends_with('e') {
        format!("{}ing", &word[..word.len()-1])
    } else if word.ends_with("ie") {
        format!("{}ying", &word[..word.len()-2])
    } else {
        format!("{}ing", word)
    }
}

fn replace_subcat(subcat : &str, members : &Vec<String>) -> String {
    let mapped_subcat = FRAMEMAP.get(subcat).unwrap_or(&subcat);
    let mut mapped_lemmas = Vec::new();
    if mapped_subcat.contains("----s") {
        for member in members {
            if member.contains(' ') {
                let parts: Vec<&str> = member.split(' ').collect();
                mapped_lemmas.push(format!("{} {}", third_person_form(parts[0]), parts[1..].join(" ")));
            } else {
                mapped_lemmas.push(third_person_form(member));
            }
        }
        mapped_subcat.replace("----s", &mapped_lemmas.join("/"))
    } else if mapped_subcat.contains("----ing") {
        for member in members {
            if member.contains(' ') {
                let parts: Vec<&str> = member.split(' ').collect();
                mapped_lemmas.push(format!("{} {}", gerund_form(parts[0]), parts[1..].join(" ")));
            } else {
                mapped_lemmas.push(gerund_form(member));
            }
        }
        mapped_subcat.replace("----ing", &mapped_lemmas.join("/"))
    } else {
        for member in members {
            mapped_lemmas.push(member.clone());
        }
        mapped_subcat.replace("----", &mapped_lemmas.join("/"))
    }
}

#[component]
pub fn Subcat(subcats : HashMap<String, Vec<String>>) -> Element {
    rsx! {
        div {
            class: "subcats",
            b { "Subcategorization frames:" },
            ul {
                for (subcat, members) in subcats.iter() {
                    li { "{replace_subcat(subcat, members)}" }
                }
            }
        }
    }
}
