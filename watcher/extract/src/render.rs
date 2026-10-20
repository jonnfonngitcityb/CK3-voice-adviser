use crate::model::{mentions_id, Collect, Person, TitleRec};
use serde::Serialize;
use serde_json::json;

const INNOVATIONS: &[&str] = &[
    "gavelkind",
    "hereditary_rule",
    "heraldry",
    "visigothic_codes",
    "primogeniture",
    "plenary_assemblies",
    "royal_prerogative",
    "casus_belli",
    "chronicle_writing",
];

const TRADITIONS: &[&str] = &[
    "practiced_pirates",
    "witenagemot",
    "ting_meet",
    "jirga",
    "table_of_princes",
];

const REALM_LAWS: &[&str] = &[
    "confederate_partition",
    "high_partition",
    "partition",
    "primogeniture",
    "ultimogeniture",
    "house_seniority",
    "feudal_elective",
    "male_only",
    "male_preference",
    "equal",
    "female_preference",
    "female_only",
    "crown_authority",
    "coronation",
    "uncrowned",
];

#[derive(Serialize)]
struct Snapshot {
    decode: String,
    source: String,
    game_version: Option<String>,
    ironman: Option<String>,
    game_date: Option<String>,
    binary_numbers: bool,
    markdown: String,
}

pub fn render(collect: &Collect, source: &str, binary_failed: bool) -> (String, String, bool) {
    let player_id = player_id(collect);
    let version = field(collect, &["version", "save_game_version"]);
    let date = field(collect, &["date", "meta_date"]);
    let ironman = field(collect, &["ironman"]);
    let body = !collect.people.is_empty() || !collect.titles.is_empty();
    // A player id in the header is not a decoded realm. Header-only stays header-only.
    let decode = if binary_failed && !body {
        "header-only"
    } else if body || player_id.is_some() {
        "gamestate"
    } else {
        "header-only"
    };

    let mut lines = Vec::new();
    lines.push(format!("Decode: {decode}"));
    lines.push(format!("Source: {source}"));
    lines.push(format!(
        "Game version: {}",
        version.clone().unwrap_or_else(|| "not in the save".to_string())
    ));
    lines.push(format!(
        "Ironman: {}",
        ironman.clone().unwrap_or_else(|| "not in the save".to_string())
    ));
    lines.push(format!(
        "Game date: {}",
        date.clone().unwrap_or_else(|| "not in the save".to_string())
    ));
    if collect.binary_numbers {
        lines.push(
            "Binary numbers use the CK3 1.5+ scale. If a number disagrees with the screen, the screen wins."
                .to_string(),
        );
    }
    if binary_failed && decode == "header-only" {
        lines.push(
            "Gamestate: not decoded. The token file is missing or the binary layout did not resolve. Header only."
                .to_string(),
        );
    }
    lines.push(String::new());

    if decode == "gamestate" {
        write_realm(&mut lines, collect, player_id.as_deref(), date.as_deref());
    }

    lines.push("# Save keys".to_string());
    if collect.top_keys.is_empty() {
        lines.push("not in the save".to_string());
    } else {
        let mut keys = collect.top_keys.clone();
        let cut = keys.len() > 40;
        keys.truncate(40);
        lines.push(keys.join(", "));
        if cut {
            lines.push("Save keys cut for size.".to_string());
        }
    }

    let mut markdown = lines.join("\n");
    markdown.push('\n');
    if markdown.len() > 150 * 1024 {
        markdown.truncate(150 * 1024);
        if let Some(end) = markdown.rfind('\n') {
            markdown.truncate(end);
        }
        markdown.push_str("\n\nCut for size.\n");
    }

    let snapshot = Snapshot {
        decode: decode.to_string(),
        source: source.to_string(),
        game_version: version,
        ironman,
        game_date: date,
        binary_numbers: collect.binary_numbers,
        markdown: markdown.clone(),
    };
    let json = serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| {
        json!({"decode": decode, "source": source}).to_string()
    });
    (markdown, json, decode == "header-only")
}

fn write_realm(lines: &mut Vec<String>, collect: &Collect, player_id: Option<&str>, date: Option<&str>) {
    let player = player_id.and_then(|id| collect.people.get(id));
    let header_name = field(collect, &["meta_player_name"]);
    let ruler_name = player
        .map(|p| p.name.clone())
        .filter(|n| !n.is_empty())
        .or(header_name)
        .unwrap_or_else(|| "not in the save".to_string());

    let held = held_titles(collect, player_id);
    let primary = primary_title(&held);

    lines.push("# Ruler".to_string());
    lines.push(format!("Ruler: {ruler_name}"));
    lines.push(format!(
        "Character id: {}",
        player_id.unwrap_or("not in the save")
    ));
    if let Some(person) = player {
        push_opt(lines, "Birth", person.birth.as_deref());
        if let (Some(birth), Some(date)) = (person.birth.as_deref(), date) {
            if let Some(years) = year_gap(date, birth) {
                lines.push(format!(
                    "Age: about {years} years, from the two dates in the save"
                ));
            }
        }
        push_opt(lines, "Female", person.female.as_deref());
        push_opt(lines, "Culture", person.culture.as_deref());
        push_opt(lines, "Faith", person.faith.as_deref());
        push_opt(lines, "House", person.house.as_deref());
        push_opt(lines, "Gold", person.gold.as_deref());
        push_opt(lines, "Prestige", person.prestige.as_deref());
        push_opt(lines, "Piety", person.piety.as_deref());
        push_opt(lines, "Renown", person.renown.as_deref());
        push_opt(lines, "Dread", person.dread.as_deref());
        push_opt(lines, "Stress", person.stress.as_deref());
        if !person.traits.is_empty() {
            lines.push(format!("Traits: {}", person.traits.join(", ")));
        }
        if !person.skills.is_empty() {
            lines.push(format!(
                "Skills: {} (list order in the save: diplomacy, martial, stewardship, intrigue, learning, prowess, when six numbers are present)",
                person.skills.join(", ")
            ));
        }
        lines.push(format!("Dynasty head: {}", dynasty_head(collect, person, player_id)));
    }
    lines.push(String::new());

    lines.push("# Titles and laws".to_string());
    if let Some(title) = primary {
        lines.push(format!(
            "Primary title: {} ({})",
            title.name.as_deref().unwrap_or(&title.key),
            title.key
        ));
        push_opt(lines, "Government", title.government.as_deref());
        push_opt(lines, "Capital", title.capital.as_deref());
        if title.laws.is_empty() {
            lines.push("Laws on that title: not in the save".to_string());
        } else {
            lines.push(format!("Laws on that title: {}", title.laws.join(", ")));
            lines.push(format!(
                "Succession law: {}",
                pick_law(&title.laws, &["confederate_partition", "high_partition", "partition", "primogeniture", "ultimogeniture", "house_seniority", "feudal_elective"])
            ));
            lines.push(format!(
                "Gender law: {}",
                pick_law(&title.laws, &["male_only", "male_preference", "equal", "female_preference", "female_only"])
            ));
            lines.push(format!(
                "Crown authority: {}",
                pick_law(&title.laws, &["crown_authority"])
            ));
            lines.push(format!(
                "Coronation law: {}",
                pick_law(&title.laws, &["coronation", "uncrowned"])
            ));
        }
    } else {
        lines.push("Primary title: not in the save".to_string());
        lines.push("Government: not in the save".to_string());
    }
    let liege = player.and_then(|p| p.liege.as_deref()).filter(|id| *id != "0");
    if let Some(id) = liege {
        lines.push(format!("Liege: {}", person_label(collect, id)));
    } else if player.is_some() {
        lines.push("Liege: none stored, or he is the top liege".to_string());
    }
    lines.push(String::new());

    lines.push("# Culture gates named on the pages".to_string());
    let culture_text = collect.cultures.join(" ");
    lines.push(format!("Innovations present: {}", present_names(&culture_text, INNOVATIONS)));
    lines.push(format!("Traditions present: {}", present_names(&culture_text, TRADITIONS)));
    lines.push("A name absent here was not found in the culture block. It is not proof the innovation is undiscovered.".to_string());
    lines.push(String::new());

    lines.push("# Domain".to_string());
    let counties: Vec<_> = held.iter().filter(|t| t.key.starts_with("c_")).collect();
    let duchies: Vec<_> = held.iter().filter(|t| t.key.starts_with("d_")).collect();
    lines.push(format!("Counties he holds: {}", counties.len()));
    lines.push(format!("Duchies he holds: {}", duchies.len()));
    lines.push(format!("Holdings he holds: {}", list_keys(&held, 40)));
    lines.push("Domain limit: not in the save. Read the domain tooltip.".to_string());
    lines.push(String::new());

    lines.push("# Family".to_string());
    if let Some(id) = player_id {
        let heir = player.and_then(|p| p.player_heir.clone()).or_else(|| field(collect, &["player_heir", "primary_heir"]));
        if let Some(heir) = heir {
            lines.push(format!(
                "Player heir stored in the save: {}. This is not the succession preview.",
                person_label(collect, &heir)
            ));
        } else {
            lines.push("Player heir: not in the save".to_string());
        }
        lines.push("Succession preview: not in the save. Read Realm, then Succession.".to_string());
        let children: Vec<_> = collect
            .people
            .values()
            .filter(|p| p.father.as_deref() == Some(id) || p.mother.as_deref() == Some(id))
            .collect();
        if children.is_empty() {
            lines.push("Children: none stored with him as father or mother".to_string());
        } else {
            for child in children {
                let traits = if child.traits.is_empty() {
                    String::new()
                } else {
                    format!(" traits {}", child.traits.join(", "))
                };
                let titles = title_list_for(collect, &child.id);
                lines.push(format!(
                    "Child: {} ({}){traits}{titles}",
                    child.name, child.id
                ));
            }
        }
        if let Some(spouse) = player.and_then(|p| p.spouse.as_deref()) {
            lines.push(format!("Spouse: {}", person_label(collect, spouse)));
        } else {
            lines.push("Spouse: not in the save".to_string());
        }
    }
    lines.push(String::new());

    lines.push("# Vassals".to_string());
    if let Some(id) = player_id {
        let vassals: Vec<_> = collect
            .people
            .values()
            .filter(|p| p.liege.as_deref() == Some(id) && p.id != id)
            .filter(|p| held_by(collect, &p.id).next().is_some())
            .take(80)
            .collect();
        if vassals.is_empty() {
            lines.push("Direct landed vassals: none stored with liege equal to him".to_string());
        } else {
            for vassal in vassals {
                let titles: Vec<&TitleRec> = held_by(collect, &vassal.id).collect();
                lines.push(format!(
                    "Vassal: {} ({}), {}",
                    vassal.name,
                    vassal.id,
                    list_keys(&titles, 8)
                ));
            }
        }
    }
    lines.push("Powerful vassals: not read as a separate list. Use the vassal lines and the faction block.".to_string());
    lines.push(String::new());

    write_filtered(lines, "# Wars", &collect.wars, collect.saw_wars, player_id, "No war in the block named his id.");
    write_filtered(lines, "# Factions", &collect.factions, collect.saw_factions, player_id, "No faction in the block named his id.");
    write_filtered(lines, "# Schemes", &collect.schemes, collect.saw_schemes, player_id, "No scheme in the block named his id.");
    write_armies(lines, collect, player_id);

    lines.push("# Claims".to_string());
    if let Some(claims) = player.and_then(|p| p.claims.as_deref()) {
        lines.push(format!("Claims he holds: {claims}"));
    } else {
        lines.push("Claims he holds: not in the save".to_string());
    }
    lines.push("Claims on his titles: not read as their own block. A claimant faction in the faction lines is the check this extractor stores.".to_string());
    lines.push(String::new());

    lines.push("# Councillors".to_string());
    lines.push("Councillors: not read. The save key list shows whether a council block was present.".to_string());
    lines.push(String::new());

    let _ = REALM_LAWS;
}

fn write_filtered(
    lines: &mut Vec<String>,
    title: &str,
    items: &[String],
    saw: bool,
    player_id: Option<&str>,
    none: &str,
) {
    lines.push(title.to_string());
    if !saw {
        lines.push("not in the save".to_string());
        lines.push(String::new());
        return;
    }
    let matched: Vec<_> = match player_id {
        Some(id) => items.iter().filter(|line| mentions_id(line, id)).cloned().collect(),
        None => items.to_vec(),
    };
    if matched.is_empty() {
        lines.push(none.to_string());
    } else {
        for line in matched.iter().take(25) {
            lines.push(line.clone());
        }
    }
    lines.push(String::new());
}

fn write_armies(lines: &mut Vec<String>, collect: &Collect, player_id: Option<&str>) {
    lines.push("# Armies".to_string());
    if !collect.saw_armies {
        lines.push("not in the save".to_string());
        lines.push(String::new());
        lines.push("# Raids".to_string());
        lines.push("Raiding army: not in the save".to_string());
        lines.push(String::new());
        return;
    }
    let his: Vec<_> = collect
        .armies
        .iter()
        .filter(|line| player_id.map(|id| mentions_id(line, id)).unwrap_or(true))
        .take(30)
        .cloned()
        .collect();
    if his.is_empty() {
        lines.push("No army in the block named his id.".to_string());
    } else {
        for line in &his {
            lines.push(line.clone());
        }
    }
    lines.push(String::new());
    lines.push("# Raids".to_string());
    let held_keys: Vec<String> = player_id
        .map(|id| held_by(collect, id).map(|t| t.key.clone()).collect())
        .unwrap_or_default();
    let raids: Vec<_> = collect
        .armies
        .iter()
        .filter(|line| line.contains("raiding=yes") || line.contains("raid"))
        .filter(|line| held_keys.iter().any(|key| line.contains(key)))
        .take(15)
        .cloned()
        .collect();
    if raids.is_empty() {
        let flagged = collect.armies.iter().any(|line| line.contains("raiding=yes"));
        if flagged {
            lines.push("Raiding army: a raiding flag was stored, and its location was not one of his holdings.".to_string());
        } else {
            lines.push("Raiding army: not in the save".to_string());
        }
    } else {
        for line in raids {
            lines.push(line);
        }
    }
    lines.push(String::new());
}

fn player_id(collect: &Collect) -> Option<String> {
    field(collect, &["player_character", "player", "currently_played_characters"])
}

fn field(collect: &Collect, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = collect.fields.get(*key) {
            let trimmed = value.split_whitespace().next().unwrap_or(value);
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn held_titles<'a>(collect: &'a Collect, player_id: Option<&str>) -> Vec<&'a TitleRec> {
    let Some(id) = player_id else {
        return Vec::new();
    };
    collect
        .titles
        .iter()
        .filter(|title| title.holder.as_deref() == Some(id))
        .collect()
}

fn held_by<'a>(collect: &'a Collect, id: &'a str) -> impl Iterator<Item = &'a TitleRec> {
    collect.titles.iter().filter(move |title| title.holder.as_deref() == Some(id))
}

fn primary_title<'a>(held: &[&'a TitleRec]) -> Option<&'a TitleRec> {
    held.iter().copied().max_by_key(|title| rank(&title.key))
}

fn rank(key: &str) -> u8 {
    if key.starts_with("e_") {
        5
    } else if key.starts_with("k_") {
        4
    } else if key.starts_with("d_") {
        3
    } else if key.starts_with("c_") {
        2
    } else if key.starts_with("b_") {
        1
    } else {
        0
    }
}

fn pick_law(laws: &[String], needles: &[&str]) -> String {
    let found: Vec<_> = laws
        .iter()
        .filter(|law| needles.iter().any(|needle| law.contains(needle)))
        .cloned()
        .collect();
    if found.is_empty() {
        "not in the save".to_string()
    } else {
        found.join(", ")
    }
}

fn present_names(text: &str, names: &[&str]) -> String {
    let found: Vec<_> = names.iter().filter(|name| text.contains(*name)).copied().collect();
    if found.is_empty() {
        "none of the named list".to_string()
    } else {
        found.join(", ")
    }
}

fn person_label(collect: &Collect, id: &str) -> String {
    match collect.people.get(id) {
        Some(person) if !person.name.is_empty() => format!("{} ({id})", person.name),
        _ => id.to_string(),
    }
}

fn dynasty_head(collect: &Collect, person: &Person, player_id: Option<&str>) -> String {
    if let Some(value) = person.dynasty_head.as_deref() {
        if value == "yes" || player_id == Some(value) {
            return "yes, stored on him".to_string();
        }
        return format!("stored value {value}");
    }
    if let Some(id) = player_id {
        if collect.dynasties.iter().any(|line| mentions_id(line, id) && (line.contains("head") || line.contains("dynast"))) {
            return "yes, a dynasty block names him as head".to_string();
        }
    }
    if collect.saw_dynasties {
        "not shown as him in the dynasty block".to_string()
    } else {
        "not in the save".to_string()
    }
}

fn title_list_for(collect: &Collect, id: &str) -> String {
    let keys: Vec<_> = held_by(collect, id).map(|t| t.key.as_str()).collect();
    if keys.is_empty() {
        String::new()
    } else {
        format!(" holds {}", keys.join(", "))
    }
}

fn list_keys(titles: &[&TitleRec], cap: usize) -> String {
    if titles.is_empty() {
        return "none stored".to_string();
    }
    let mut keys: Vec<_> = titles.iter().map(|t| t.key.as_str()).collect();
    let more = keys.len() > cap;
    keys.truncate(cap);
    let mut line = keys.join(", ");
    if more {
        line.push_str(" ...");
    }
    line
}

fn push_opt(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    match value {
        Some(value) if !value.is_empty() => lines.push(format!("{label}: {value}")),
        _ => lines.push(format!("{label}: not in the save")),
    }
}

fn year_gap(date: &str, birth: &str) -> Option<i32> {
    let date_year = date.split('.').next()?.parse::<i32>().ok()?;
    let birth_year = birth.split('.').next()?.parse::<i32>().ok()?;
    Some(date_year - birth_year)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Person;

    #[test]
    fn cuts_a_long_snapshot() {
        let mut collect = Collect::default();
        collect.set_field("player_character", "1".to_string());
        collect.people.insert(
            "1".to_string(),
            Person {
                id: "1".to_string(),
                name: "Charles".to_string(),
                claims: Some("x".repeat(200_000)),
                ..Person::default()
            },
        );
        let (markdown, _, header_only) = render(&collect, "test", false);
        assert!(!header_only);
        assert!(markdown.contains("Cut for size."));
        assert!(markdown.len() < 160 * 1024);
    }

    #[test]
    fn header_only_stays_header_only_with_a_player_id() {
        let mut collect = Collect::default();
        collect.set_field("player_character", "1".to_string());
        collect.set_field("version", "1.20.1".to_string());
        let (markdown, _, header_only) = render(&collect, "autosave.ck3", true);
        assert!(header_only);
        assert!(markdown.contains("Decode: header-only"));
        assert!(markdown.contains("1.20.1"));
    }
}
