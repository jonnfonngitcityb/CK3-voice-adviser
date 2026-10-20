use crate::model::{
    class_of, is_skippable_person_key, Class, Collect, Person, Section, TitleRec,
};
use jomini::binary::{Token, TokenReader};
use std::collections::{HashMap, VecDeque};

pub fn parse_binary(
    data: &[u8],
    names: &HashMap<u16, String>,
    collect: &mut Collect,
) -> Result<(), String> {
    collect.binary_numbers = true;
    let mut walker = Walker {
        reader: TokenReader::from_slice(data),
        names,
        buf: VecDeque::new(),
        collect,
    };
    walker.parse_body(true)
}

struct Walker<'a> {
    reader: TokenReader<'a>,
    names: &'a HashMap<u16, String>,
    buf: VecDeque<Ev>,
    collect: &'a mut Collect,
}

#[derive(Clone, Debug)]
enum Ev {
    Open,
    Close,
    Equal,
    Word(String),
}

impl<'a> Walker<'a> {
    fn parse_body(&mut self, record: bool) -> Result<(), String> {
        loop {
            match self.pop_ev()? {
                None => return Ok(()),
                Some(Ev::Close) => return Ok(()),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => match self.classify_open()? {
                    Shape::Empty => {}
                    Shape::Object => self.parse_body(record)?,
                    Shape::List => self.skip_depth(1)?,
                },
                Some(Ev::Word(key)) => match self.pop_ev()? {
                    Some(Ev::Equal) => self.handle(&key, record)?,
                    Some(Ev::Open) => {
                        self.buf.push_front(Ev::Open);
                        self.handle(&key, record)?;
                    }
                    Some(other) => {
                        self.buf.push_front(other);
                    }
                    None => return Ok(()),
                },
            }
        }
    }

    fn handle(&mut self, key: &str, record: bool) -> Result<(), String> {
        if record {
            self.collect.remember_top(key);
            if key.starts_with("unk_") {
                // counted when the id was resolved
            } else {
                self.collect.resolved_binary_keys =
                    self.collect.resolved_binary_keys.saturating_add(1);
            }
        }
        match class_of(key) {
            Class::Skip => self.skip_value(),
            Class::Field => {
                let value = self.read_scalar()?;
                self.collect.set_field(key, value);
                Ok(())
            }
            Class::People => self.take_people(),
            Class::Titles => self.take_titles(),
            Class::Blobs(section) => self.take_blobs(section),
            Class::Named(section) => self.take_named(section),
            Class::Descend => {
                if matches!(self.peek_ev()?, Some(Ev::Open)) {
                    let _ = self.pop_ev()?;
                    match self.classify_open()? {
                        Shape::Empty => Ok(()),
                        Shape::Object => self.parse_body(true),
                        Shape::List => self.skip_depth(1),
                    }
                } else {
                    self.skip_value()
                }
            }
            Class::Unknown => self.skip_value(),
        }
    }

    fn take_people(&mut self) -> Result<(), String> {
        self.collect.saw_people = true;
        if !self.begin_container()? {
            return Ok(());
        }
        loop {
            if self.collect.people.len() >= 50_000 {
                return self.skip_depth(1);
            }
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(()),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => self.skip_depth(1)?,
                Some(Ev::Word(id)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                        let mut person = self.parse_person(id)?;
                        if person.name.is_empty()
                            && person.birth.is_none()
                            && person.liege.is_none()
                            && person.father.is_none()
                        {
                            continue;
                        }
                        if person.name.is_empty() {
                            person.name = person.id.clone();
                        }
                        self.collect.people.insert(person.id.clone(), person);
                    }
                }
            }
        }
    }

    fn parse_person(&mut self, id: String) -> Result<Person, String> {
        if !matches!(self.peek_ev()?, Some(Ev::Open)) {
            self.skip_value()?;
            return Ok(Person {
                id,
                ..Person::default()
            });
        }
        let _ = self.pop_ev()?;
        let mut person = Person {
            id,
            ..Person::default()
        };
        loop {
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(person),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => self.skip_depth(1)?,
                Some(Ev::Word(key)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                    }
                    if is_skippable_person_key(&key) {
                        self.skip_value()?;
                        continue;
                    }
                    self.apply_person(&mut person, &key)?;
                }
            }
        }
    }

    fn apply_person(&mut self, person: &mut Person, key: &str) -> Result<(), String> {
        match key {
            "first_name" | "name" => person.name = self.read_scalar()?,
            "birth" | "birth_date" => person.birth = Some(self.read_scalar()?),
            "female" => person.female = Some(self.read_scalar()?),
            "culture" => person.culture = Some(self.read_scalar()?),
            "faith" => person.faith = Some(self.read_scalar()?),
            "dynasty_house" | "dynasty" => person.house = Some(self.read_scalar()?),
            "liege" | "employer" => person.liege = Some(self.read_scalar()?),
            "father" => person.father = Some(self.read_scalar()?),
            "mother" => person.mother = Some(self.read_scalar()?),
            "spouse" | "primary_spouse" => person.spouse = Some(self.read_scalar()?),
            "player_heir" | "primary_heir" => person.player_heir = Some(self.read_scalar()?),
            "dynasty_head" | "is_dynast" | "house_head" => {
                person.dynasty_head = Some(self.read_scalar()?)
            }
            "claims" | "claim" | "pressed_claims" => person.claims = Some(cap_text(self.read_scalar()?)),
            "traits" => person.traits = split_words(&self.read_scalar()?),
            "skill" | "skills" => person.skills = split_words(&self.read_scalar()?),
            "gold" => person.gold = Some(self.read_scalar()?),
            "prestige" => person.prestige = Some(self.read_scalar()?),
            "piety" => person.piety = Some(self.read_scalar()?),
            "renown" => person.renown = Some(self.read_scalar()?),
            "dread" => person.dread = Some(self.read_scalar()?),
            "stress" => person.stress = Some(self.read_scalar()?),
            "alive_data" => self.fill_alive(person)?,
            _ => self.skip_value()?,
        }
        Ok(())
    }

    fn fill_alive(&mut self, person: &mut Person) -> Result<(), String> {
        if !matches!(self.peek_ev()?, Some(Ev::Open)) {
            return self.skip_value();
        }
        let _ = self.pop_ev()?;
        loop {
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(()),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => self.skip_depth(1)?,
                Some(Ev::Word(key)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                    }
                    let value = self.read_scalar()?;
                    match key.as_str() {
                        "gold" | "wealth" => person.gold = Some(value),
                        "prestige" => person.prestige = Some(value),
                        "piety" => person.piety = Some(value),
                        "renown" => person.renown = Some(value),
                        "dread" => person.dread = Some(value),
                        "stress" => person.stress = Some(value),
                        "liege" if person.liege.is_none() => person.liege = Some(value),
                        _ => {}
                    }
                }
            }
        }
    }

    fn take_titles(&mut self) -> Result<(), String> {
        self.collect.saw_titles = true;
        if !self.begin_container()? {
            return Ok(());
        }
        loop {
            if self.collect.titles.len() >= 40_000 {
                return self.skip_depth(1);
            }
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(()),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => self.skip_depth(1)?,
                Some(Ev::Word(key)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                    }
                    let title = self.parse_title(key)?;
                    self.collect.titles.push(title);
                }
            }
        }
    }

    fn parse_title(&mut self, key: String) -> Result<TitleRec, String> {
        if !matches!(self.peek_ev()?, Some(Ev::Open)) {
            self.skip_value()?;
            return Ok(TitleRec {
                key,
                ..TitleRec::default()
            });
        }
        let _ = self.pop_ev()?;
        let mut title = TitleRec {
            key,
            ..TitleRec::default()
        };
        loop {
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(title),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => self.skip_depth(1)?,
                Some(Ev::Word(field)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                    }
                    match field.as_str() {
                        "history" => self.skip_value()?,
                        "holder" | "holder_id" => title.holder = Some(self.read_scalar()?),
                        "capital" | "capital_barony" | "capital_county" => {
                            title.capital = Some(self.read_scalar()?)
                        }
                        "government" => title.government = Some(self.read_scalar()?),
                        "name" | "key" => {
                            let value = self.read_scalar()?;
                            if title.name.is_none() {
                                title.name = Some(value);
                            }
                        }
                        "laws" | "succession" | "succession_laws" => {
                            title.laws.extend(split_words(&self.read_scalar()?));
                        }
                        "de_jure_liege" | "liege" | "de_facto_liege" => {
                            title.liege = Some(self.read_scalar()?)
                        }
                        _ => self.skip_value()?,
                    }
                }
            }
        }
    }

    fn take_blobs(&mut self, section: Section) -> Result<(), String> {
        self.collect.mark_section(section);
        if !self.begin_container()? {
            return Ok(());
        }
        let mut kept = 0;
        loop {
            match self.pop_ev()? {
                Some(Ev::Close) | None => return Ok(()),
                Some(Ev::Equal) => {}
                Some(Ev::Open) => {
                    if kept >= 400 {
                        self.skip_depth(1)?;
                    } else if let Some(line) = self.summary_line(None)? {
                        self.collect.push_section(section, line);
                        kept += 1;
                    }
                }
                Some(Ev::Word(key)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                    }
                    if kept >= 400 {
                        self.skip_value()?;
                        continue;
                    }
                    if matches!(self.peek_ev()?, Some(Ev::Open)) {
                        let _ = self.pop_ev()?;
                        if let Some(line) = self.summary_line(Some(&key))? {
                            self.collect.push_section(section, line);
                            kept += 1;
                        }
                    } else {
                        let _ = self.read_scalar()?;
                    }
                }
            }
        }
    }

    fn take_named(&mut self, section: Section) -> Result<(), String> {
        self.take_blobs(section)
    }

    fn summary_line(&mut self, key: Option<&str>) -> Result<Option<String>, String> {
        let summary = self.shallow_summary()?;
        if summary.is_empty() {
            return Ok(None);
        }
        let line = match key {
            Some(key) => format!("{key}: {summary}"),
            None => summary,
        };
        Ok(Some(cap_text(line)))
    }

    fn shallow_summary(&mut self) -> Result<String, String> {
        let mut parts = Vec::new();
        loop {
            match self.pop_ev()? {
                Some(Ev::Close) | None => break,
                Some(Ev::Equal) => {}
                Some(Ev::Open) => {
                    let inner = self.shallow_summary()?;
                    if !inner.is_empty() && parts_len(&parts) < 400 {
                        parts.push(inner);
                    }
                }
                Some(Ev::Word(key)) => {
                    if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                        let _ = self.pop_ev()?;
                        if matches!(self.peek_ev()?, Some(Ev::Open)) {
                            let _ = self.pop_ev()?;
                            let inner = self.shallow_summary()?;
                            if !inner.is_empty() && parts_len(&parts) < 400 {
                                parts.push(format!("{key}={inner}"));
                            }
                        } else if let Some(Ev::Word(value)) = self.pop_ev()? {
                            if parts_len(&parts) < 400 {
                                parts.push(format!("{key}={value}"));
                            }
                        }
                    } else if parts_len(&parts) < 400 {
                        parts.push(key);
                    }
                }
            }
        }
        Ok(parts.join(" "))
    }

    fn read_scalar(&mut self) -> Result<String, String> {
        match self.pop_ev()? {
            Some(Ev::Word(word)) => Ok(word),
            Some(Ev::Open) => {
                let mut parts = Vec::new();
                loop {
                    match self.pop_ev()? {
                        Some(Ev::Close) | None => break,
                        Some(Ev::Equal) => {}
                        Some(Ev::Open) => self.skip_depth(1)?,
                        Some(Ev::Word(word)) => {
                            if matches!(self.peek_ev()?, Some(Ev::Equal)) {
                                let _ = self.pop_ev()?;
                                self.skip_value()?;
                            } else if parts_len(&parts) < 400 {
                                parts.push(word);
                            }
                        }
                    }
                }
                Ok(parts.join(" "))
            }
            Some(Ev::Close) => {
                self.buf.push_front(Ev::Close);
                Ok(String::new())
            }
            Some(Ev::Equal) | None => Ok(String::new()),
        }
    }

    fn begin_container(&mut self) -> Result<bool, String> {
        match self.pop_ev()? {
            Some(Ev::Open) => Ok(true),
            Some(Ev::Close) => {
                self.buf.push_front(Ev::Close);
                Ok(false)
            }
            Some(_) | None => Ok(false),
        }
    }

    fn skip_value(&mut self) -> Result<(), String> {
        if self.buf.is_empty() {
            return match self.read_ev()? {
                Some(Ev::Open) => self
                    .reader
                    .skip_container()
                    .map_err(|err| err.to_string()),
                Some(_) => Ok(()),
                None => Err("missing value".to_string()),
            };
        }
        match self.buf.pop_front() {
            Some(Ev::Open) => self.skip_depth(1),
            Some(_) => Ok(()),
            None => Err("missing value".to_string()),
        }
    }

    fn skip_depth(&mut self, mut depth: i32) -> Result<(), String> {
        while depth > 0 {
            match self.pop_ev()? {
                Some(Ev::Open) => depth += 1,
                Some(Ev::Close) => depth -= 1,
                Some(_) => {}
                None => return Err("unbalanced binary container".to_string()),
            }
        }
        Ok(())
    }

    fn classify_open(&mut self) -> Result<Shape, String> {
        let Some(first) = self.pop_ev()? else {
            return Err("truncated container".to_string());
        };
        if matches!(first, Ev::Close) {
            return Ok(Shape::Empty);
        }
        let Some(second) = self.pop_ev()? else {
            self.buf.push_front(first);
            return Ok(Shape::List);
        };
        let object = matches!((&first, &second), (Ev::Word(_), Ev::Equal));
        self.buf.push_front(second);
        self.buf.push_front(first);
        Ok(if object { Shape::Object } else { Shape::List })
    }

    fn peek_ev(&mut self) -> Result<Option<Ev>, String> {
        if self.buf.is_empty() {
            if let Some(ev) = self.read_ev()? {
                self.buf.push_back(ev);
            }
        }
        Ok(self.buf.front().cloned())
    }

    fn pop_ev(&mut self) -> Result<Option<Ev>, String> {
        if let Some(ev) = self.buf.pop_front() {
            return Ok(Some(ev));
        }
        self.read_ev()
    }

    fn read_ev(&mut self) -> Result<Option<Ev>, String> {
        let next = self.reader.next().map_err(|err| err.to_string())?;
        let Some(token) = next else {
            return Ok(None);
        };
        let ev = match token {
            Token::Open => Ev::Open,
            Token::Close => Ev::Close,
            Token::Equal => Ev::Equal,
            Token::Id(id) => Ev::Word(self.resolve(id)),
            Token::U32(value) => Ev::Word(value.to_string()),
            Token::U64(value) => Ev::Word(value.to_string()),
            Token::I32(value) => Ev::Word(value.to_string()),
            Token::I64(value) => Ev::Word(value.to_string()),
            Token::Bool(value) => Ev::Word(if value { "yes" } else { "no" }.to_string()),
            Token::Quoted(scalar) | Token::Unquoted(scalar) => {
                Ev::Word(String::from_utf8_lossy(scalar.as_bytes()).into_owned())
            }
            Token::F32(bytes) => Ev::Word(fmt_f32(bytes)),
            Token::F64(bytes) => Ev::Word(fmt_f64(bytes)),
            Token::Rgb(_) => Ev::Word("rgb".to_string()),
            Token::Lookup(index) => Ev::Word(format!("lookup_{index}")),
        };
        Ok(Some(ev))
    }

    fn resolve(&mut self, id: u16) -> String {
        if let Some(name) = self.names.get(&id) {
            return name.clone();
        }
        self.collect.unknown_tokens = self.collect.unknown_tokens.saturating_add(1);
        format!("unk_{id}")
    }
}

enum Shape {
    Empty,
    Object,
    List,
}

fn split_words(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn parts_len(parts: &[String]) -> usize {
    parts.iter().map(String::len).sum()
}

fn cap_text(mut text: String) -> String {
    if text.chars().count() > 400 {
        text = text.chars().take(400).collect();
        text.push_str("...");
    }
    text
}

/// CK3 1.5+ f64. The raw i64 is a 5-decimal fixed point.
pub fn fmt_f64(bytes: [u8; 8]) -> String {
    let value = i64::from_le_bytes(bytes) as f64;
    let eps = f64::from(f32::EPSILON);
    let scaled = (value + (eps * value.signum())).trunc() / 100_000.0;
    fmt_num(scaled)
}

pub fn fmt_f32(bytes: [u8; 4]) -> String {
    fmt_num(f64::from(f32::from_bits(u32::from_le_bytes(bytes))))
}

fn fmt_num(value: f64) -> String {
    if !value.is_finite() {
        return "not-a-number".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    let text = format!("{rounded:.3}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jomini::Scalar;

    fn names() -> HashMap<u16, String> {
        let mut map = HashMap::new();
        for (id, name) in [
            (0x1001, "date"),
            (0x1002, "player_character"),
            (0x1003, "living"),
            (0x1004, "first_name"),
            (0x1005, "gold"),
            (0x1006, "landed_titles"),
            (0x1007, "holder"),
            (0x1008, "dna"),
        ] {
            map.insert(id, name.to_string());
        }
        map
    }

    fn encode(tokens: &[Token<'_>]) -> Vec<u8> {
        let mut out = Vec::new();
        for token in tokens {
            token.write(&mut out).unwrap();
        }
        out
    }

    #[test]
    fn reads_a_binary_realm_and_flavor15_gold() {
        let date = Scalar::new(b"867.1.1");
        let charles = Scalar::new(b"Charles");
        let title = Scalar::new(b"k_france");
        let bytes = encode(&[
            Token::Id(0x1001),
            Token::Equal,
            Token::Unquoted(date),
            Token::Id(0x1002),
            Token::Equal,
            Token::U32(1),
            Token::Id(0x1003),
            Token::Equal,
            Token::Open,
            Token::U32(1),
            Token::Equal,
            Token::Open,
            Token::Id(0x1004),
            Token::Equal,
            Token::Quoted(charles),
            Token::Id(0x1005),
            Token::Equal,
            Token::F64(4_250_000i64.to_le_bytes()),
            Token::Id(0x1008),
            Token::Equal,
            Token::Open,
            Token::U32(9),
            Token::Equal,
            Token::U32(9),
            Token::Close,
            Token::Close,
            Token::Close,
            Token::Id(0x1006),
            Token::Equal,
            Token::Open,
            Token::Unquoted(title),
            Token::Equal,
            Token::Open,
            Token::Id(0x1007),
            Token::Equal,
            Token::U32(1),
            Token::Close,
            Token::Close,
            Token::Id(0xABCD),
            Token::Equal,
            Token::U32(1),
        ]);
        let mut collect = Collect::default();
        parse_binary(&bytes, &names(), &mut collect).unwrap();
        assert_eq!(collect.fields.get("date").map(String::as_str), Some("867.1.1"));
        assert_eq!(collect.fields.get("player_character").map(String::as_str), Some("1"));
        assert_eq!(collect.people.get("1").unwrap().name, "Charles");
        assert_eq!(collect.people.get("1").unwrap().gold.as_deref(), Some("42.5"));
        let france = collect.titles.iter().find(|title| title.key == "k_france").unwrap();
        assert_eq!(france.holder.as_deref(), Some("1"));
        assert!(collect.top_keys.iter().any(|key| key == "unk_43981"));
        assert!(collect.unknown_tokens >= 1);
        assert!(collect.resolved_binary_keys >= 4);
    }

    #[test]
    fn an_unresolved_token_map_does_not_invent_a_player() {
        let date = Scalar::new(b"867.1.1");
        let bytes = encode(&[
            Token::Id(0x1002),
            Token::Equal,
            Token::U32(1),
            Token::Id(0x1003),
            Token::Equal,
            Token::Open,
            Token::U32(1),
            Token::Equal,
            Token::Open,
            Token::Id(0x1004),
            Token::Equal,
            Token::Unquoted(date),
            Token::Close,
            Token::Close,
        ]);
        let mut collect = Collect::default();
        parse_binary(&bytes, &HashMap::new(), &mut collect).unwrap();
        assert!(collect.people.is_empty());
        assert!(collect.fields.get("player_character").is_none());
        assert_eq!(collect.resolved_binary_keys, 0);
        assert!(collect.unknown_tokens >= 1);
        assert!(collect.top_keys.iter().all(|key| key.starts_with("unk_")));
    }
}
