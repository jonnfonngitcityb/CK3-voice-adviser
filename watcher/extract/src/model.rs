use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    War,
    Faction,
    Scheme,
    Army,
    Culture,
    Faith,
    Dynasty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Skip,
    Field,
    People,
    Titles,
    Blobs(Section),
    Named(Section),
    Descend,
    Unknown,
}

pub fn class_of(key: &str) -> Class {
    match key {
        "history" | "dead" | "dead_unprunable" | "provinces" | "coat_of_arms" | "dna"
        | "genes" | "variables" | "game_rules" | "activity_manager" | "opinions"
        | "artifacts" | "stories" | "played_character_history" | "army_regiments"
        | "regiments" => Class::Skip,
        "date" | "version" | "player_character" | "player" | "ironman" | "meta_player_name"
        | "meta_date" | "meta_title_name" | "player_heir" | "primary_heir"
        | "save_game_version" | "currently_played_characters" => Class::Field,
        "living" | "characters" => Class::People,
        "landed_titles" => Class::Titles,
        "wars" | "active_wars" => Class::Blobs(Section::War),
        "factions" => Class::Blobs(Section::Faction),
        "schemes" | "scheme_manager" => Class::Blobs(Section::Scheme),
        "armies" | "units" | "army_manager" => Class::Blobs(Section::Army),
        "cultures" => Class::Named(Section::Culture),
        "faiths" | "religions" => Class::Named(Section::Faith),
        "dynasties" | "dynasty_houses" => Class::Named(Section::Dynasty),
        "meta_data" | "character" | "database" | "culture_manager" | "faith_manager"
        | "religion_manager" | "dynasty_manager" | "title_manager" | "war_manager"
        | "faction_manager" | "military" | "units_manager" => Class::Descend,
        _ => Class::Unknown,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub female: Option<String>,
    pub culture: Option<String>,
    pub faith: Option<String>,
    pub house: Option<String>,
    pub liege: Option<String>,
    pub father: Option<String>,
    pub mother: Option<String>,
    pub spouse: Option<String>,
    pub player_heir: Option<String>,
    pub traits: Vec<String>,
    pub skills: Vec<String>,
    pub gold: Option<String>,
    pub prestige: Option<String>,
    pub piety: Option<String>,
    pub renown: Option<String>,
    pub dread: Option<String>,
    pub stress: Option<String>,
    pub birth: Option<String>,
    pub dynasty_head: Option<String>,
    pub claims: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TitleRec {
    pub key: String,
    pub holder: Option<String>,
    pub capital: Option<String>,
    pub government: Option<String>,
    pub name: Option<String>,
    pub laws: Vec<String>,
    pub liege: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Collect {
    pub fields: BTreeMap<String, String>,
    pub top_keys: Vec<String>,
    pub people: BTreeMap<String, Person>,
    pub titles: Vec<TitleRec>,
    pub wars: Vec<String>,
    pub factions: Vec<String>,
    pub schemes: Vec<String>,
    pub armies: Vec<String>,
    pub cultures: Vec<String>,
    pub faiths: Vec<String>,
    pub dynasties: Vec<String>,
    pub saw_people: bool,
    pub saw_titles: bool,
    pub saw_wars: bool,
    pub saw_factions: bool,
    pub saw_schemes: bool,
    pub saw_armies: bool,
    pub saw_cultures: bool,
    pub saw_faiths: bool,
    pub saw_dynasties: bool,
    pub unknown_tokens: u32,
    pub resolved_binary_keys: u32,
    pub binary_numbers: bool,
}

impl Collect {
    pub fn remember_top(&mut self, key: &str) {
        if !self.top_keys.iter().any(|k| k == key) {
            self.top_keys.push(key.to_string());
        }
    }

    pub fn set_field(&mut self, key: &str, value: String) {
        if !value.is_empty() && !self.fields.contains_key(key) {
            self.fields.insert(key.to_string(), value);
        }
    }

    pub fn push_section(&mut self, section: Section, line: String) {
        if line.is_empty() {
            return;
        }
        let (flag, list) = match section {
            Section::War => (&mut self.saw_wars, &mut self.wars),
            Section::Faction => (&mut self.saw_factions, &mut self.factions),
            Section::Scheme => (&mut self.saw_schemes, &mut self.schemes),
            Section::Army => (&mut self.saw_armies, &mut self.armies),
            Section::Culture => (&mut self.saw_cultures, &mut self.cultures),
            Section::Faith => (&mut self.saw_faiths, &mut self.faiths),
            Section::Dynasty => (&mut self.saw_dynasties, &mut self.dynasties),
        };
        *flag = true;
        if list.len() < 400 {
            list.push(line);
        }
    }

    pub fn mark_section(&mut self, section: Section) {
        match section {
            Section::War => self.saw_wars = true,
            Section::Faction => self.saw_factions = true,
            Section::Scheme => self.saw_schemes = true,
            Section::Army => self.saw_armies = true,
            Section::Culture => self.saw_cultures = true,
            Section::Faith => self.saw_faiths = true,
            Section::Dynasty => self.saw_dynasties = true,
        }
    }
}

pub fn is_skippable_person_key(key: &str) -> bool {
    matches!(
        key,
        "dna" | "genes" | "coat_of_arms"
            | "portrait_data"
            | "portrait_info"
            | "family_data"
            | "dead_data"
            | "artifacts"
            | "claims_history"
    )
}

pub fn mentions_id(text: &str, id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| word == id)
}
