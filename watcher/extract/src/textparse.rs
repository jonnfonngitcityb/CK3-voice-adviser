use crate::model::{
    class_of, is_skippable_person_key, Class, Collect, Person, Section, TitleRec,
};

pub fn parse_text(input: &str, collect: &mut Collect) -> Result<(), String> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        index: 0,
    };
    parser.skip_ws();
    if parser.peek() == Some(b'{') {
        parser.bump();
        parser.object_body(true, collect)?;
        return Ok(());
    }
    parser.object_body(false, collect)?;
    Ok(())
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn bump(&mut self) {
        if self.index < self.bytes.len() {
            self.index += 1;
        }
    }

    fn skip_ws(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                self.bump();
            }
            if self.peek() == Some(b'#') {
                while self.peek().is_some() && self.peek() != Some(b'\n') {
                    self.bump();
                }
                continue;
            }
            break;
        }
    }

    fn eof(&self) -> bool {
        self.index >= self.bytes.len()
    }

    fn read_word(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.peek() == Some(b'"') {
            return self.read_quoted();
        }
        let start = self.index;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || matches!(byte, b'=' | b'{' | b'}' | b'<' | b'>') {
                break;
            }
            self.bump();
        }
        if start == self.index {
            return Err("expected a word".to_string());
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.index]).into_owned())
    }

    fn read_quoted(&mut self) -> Result<String, String> {
        self.bump();
        let mut out = String::new();
        while let Some(byte) = self.peek() {
            self.bump();
            if byte == b'"' {
                return Ok(out);
            }
            if byte == b'\\' {
                if let Some(next) = self.peek() {
                    self.bump();
                    out.push(next as char);
                }
                continue;
            }
            out.push(byte as char);
        }
        Err("unterminated string".to_string())
    }

    fn skip_value(&mut self) -> Result<(), String> {
        self.skip_ws();
        if self.peek() == Some(b'{') {
            self.bump();
            let mut depth = 1;
            let mut in_string = false;
            let mut escape = false;
            while depth > 0 {
                let Some(byte) = self.peek() else {
                    return Err("unbalanced brace".to_string());
                };
                self.bump();
                if in_string {
                    if escape {
                        escape = false;
                    } else if byte == b'\\' {
                        escape = true;
                    } else if byte == b'"' {
                        in_string = false;
                    }
                    continue;
                }
                match byte {
                    b'"' => in_string = true,
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
            }
            return Ok(());
        }
        if self.peek() == Some(b'"') {
            self.read_quoted()?;
            return Ok(());
        }
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || matches!(byte, b'{' | b'}') {
                break;
            }
            self.bump();
        }
        Ok(())
    }

    fn read_scalar(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.peek() == Some(b'{') {
            self.bump();
            let mut parts = Vec::new();
            loop {
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.bump();
                    break;
                }
                if self.eof() {
                    return Err("unbalanced list".to_string());
                }
                let word = self.read_word()?;
                self.skip_ws();
                if self.peek() == Some(b'=') {
                    self.bump();
                    self.skip_value()?;
                    continue;
                }
                if self.peek() == Some(b'{') {
                    self.skip_value()?;
                    continue;
                }
                parts.push(word);
            }
            return Ok(parts.join(" "));
        }
        self.read_word()
    }

    fn object_body(&mut self, braced: bool, collect: &mut Collect) -> Result<(), String> {
        loop {
            self.skip_ws();
            if self.eof() || self.peek() == Some(b'}') {
                if braced && self.peek() == Some(b'}') {
                    self.bump();
                }
                return Ok(());
            }
            let key = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
            } else if self.peek() == Some(b'<') || self.peek() == Some(b'>') {
                while matches!(self.peek(), Some(b'<' | b'>' | b'=')) {
                    self.bump();
                }
            }
            self.handle(&key, true, collect)?;
        }
    }

    fn handle(&mut self, key: &str, top: bool, collect: &mut Collect) -> Result<(), String> {
        if top {
            collect.remember_top(key);
        }
        match class_of(key) {
            Class::Skip => self.skip_value(),
            Class::Field => {
                let value = self.read_scalar()?;
                collect.set_field(key, value);
                Ok(())
            }
            Class::People => self.take_people(collect),
            Class::Titles => self.take_titles(collect),
            Class::Blobs(section) => self.take_blobs(section, collect),
            Class::Named(section) => self.take_named(section, collect),
            Class::Descend => self.descend(collect),
            Class::Unknown if top => {
                self.skip_value()?;
                Ok(())
            }
            Class::Unknown => self.skip_value(),
        }
    }

    fn descend(&mut self, collect: &mut Collect) -> Result<(), String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(());
        }
        self.bump();
        self.object_body(true, collect)
    }

    fn take_people(&mut self, collect: &mut Collect) -> Result<(), String> {
        collect.saw_people = true;
        self.each_object(|parser, id, collect| {
            let mut person = parser.parse_person(id)?;
            if person.name.is_empty()
                && person.birth.is_none()
                && person.liege.is_none()
                && person.father.is_none()
            {
                return Ok(());
            }
            if person.name.is_empty() {
                person.name = person.id.clone();
            }
            collect.people.insert(person.id.clone(), person);
            Ok(())
        }, collect)
    }

    fn parse_person(&mut self, id: String) -> Result<Person, String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(Person {
                id,
                ..Person::default()
            });
        }
        self.bump();
        let mut person = Person {
            id,
            ..Person::default()
        };
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced character".to_string());
            }
            let key = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
            }
            if is_skippable_person_key(&key) {
                self.skip_value()?;
                continue;
            }
            match key.as_str() {
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
                "claims" | "claim" | "pressed_claims" => person.claims = Some(self.read_scalar()?),
                "traits" => person.traits = split_words(&self.read_scalar()?),
                "skill" | "skills" => person.skills = split_words(&self.read_scalar()?),
                "gold" => person.gold = Some(self.read_scalar()?),
                "prestige" => person.prestige = Some(self.read_scalar()?),
                "piety" => person.piety = Some(self.read_scalar()?),
                "renown" => person.renown = Some(self.read_scalar()?),
                "dread" => person.dread = Some(self.read_scalar()?),
                "stress" => person.stress = Some(self.read_scalar()?),
                "alive_data" => self.fill_alive(&mut person)?,
                _ => self.skip_value()?,
            }
        }
        Ok(person)
    }

    fn fill_alive(&mut self, person: &mut Person) -> Result<(), String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(());
        }
        self.bump();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced alive_data".to_string());
            }
            let key = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
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
        Ok(())
    }

    fn take_titles(&mut self, collect: &mut Collect) -> Result<(), String> {
        collect.saw_titles = true;
        self.each_object(|parser, key, collect| {
            let title = parser.parse_title(key)?;
            collect.titles.push(title);
            Ok(())
        }, collect)
    }

    fn parse_title(&mut self, key: String) -> Result<TitleRec, String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(TitleRec {
                key,
                ..TitleRec::default()
            });
        }
        self.bump();
        let mut title = TitleRec {
            key,
            ..TitleRec::default()
        };
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced title".to_string());
            }
            let field = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
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
        Ok(title)
    }

    fn take_blobs(&mut self, section: Section, collect: &mut Collect) -> Result<(), String> {
        collect.mark_section(section);
        self.walk_blobs(section, collect)
    }

    fn walk_blobs(&mut self, section: Section, collect: &mut Collect) -> Result<(), String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(());
        }
        self.bump();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced block".to_string());
            }
            let key = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
            }
            self.skip_ws();
            if self.peek() == Some(b'{') {
                let summary = self.shallow_summary()?;
                if summary.is_empty() {
                    continue;
                }
                if summary.starts_with('\u{0}') {
                    continue;
                }
                collect.push_section(section, format!("{key}: {summary}"));
            } else {
                let _ = self.read_scalar()?;
            }
        }
        Ok(())
    }

    fn shallow_summary(&mut self) -> Result<String, String> {
        self.bump();
        let mut parts = Vec::new();
        let mut nested = false;
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced summary".to_string());
            }
            let key = self.read_word()?;
            self.skip_ws();
            let has_eq = self.peek() == Some(b'=');
            if has_eq {
                self.bump();
            }
            self.skip_ws();
            if self.peek() == Some(b'{') {
                let inner = self.shallow_summary()?;
                if inner.is_empty() && !has_eq {
                    nested = true;
                } else if !inner.is_empty() {
                    parts.push(format!("{key}={inner}"));
                } else {
                    nested = true;
                }
            } else if has_eq {
                let value = self.read_word()?;
                parts.push(format!("{key}={value}"));
            } else {
                parts.push(key);
            }
        }
        if parts.is_empty() && nested {
            return Ok(String::new());
        }
        let mut line = parts.join(" ");
        if line.chars().count() > 400 {
            line = line.chars().take(400).collect();
            line.push_str("...");
        }
        Ok(line)
    }

    fn take_named(&mut self, section: Section, collect: &mut Collect) -> Result<(), String> {
        collect.mark_section(section);
        self.each_object(|parser, key, collect| {
            parser.skip_ws();
            if parser.peek() != Some(b'{') {
                parser.skip_value()?;
                return Ok(());
            }
            let summary = parser.shallow_summary()?;
            if !summary.is_empty() {
                collect.push_section(section, format!("{key}: {summary}"));
            }
            Ok(())
        }, collect)
    }

    fn each_object(
        &mut self,
        mut on_entry: impl FnMut(&mut Self, String, &mut Collect) -> Result<(), String>,
        collect: &mut Collect,
    ) -> Result<(), String> {
        self.skip_ws();
        if self.peek() != Some(b'{') {
            self.skip_value()?;
            return Ok(());
        }
        self.bump();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.bump();
                break;
            }
            if self.eof() {
                return Err("unbalanced container".to_string());
            }
            let key = self.read_word()?;
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
            }
            on_entry(self, key, collect)?;
        }
        Ok(())
    }
}

fn split_words(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
meta_data={
  version="1.20.1"
  meta_player_name="Charles"
  ironman=yes
}
date=867.1.1
player_character=1
living={
  1={
    first_name="Charles"
    birth=846.6.13
    female=no
    culture=french
    faith=catholic
    liege=0
    skill={ 8 7 6 5 9 4 }
    traits={ brave just }
    alive_data={ gold=42.5 prestige=100 }
    player_heir=2
    spouse=3
    dna={ hidden=1 }
  }
  2={ first_name="Louis" father=1 liege=1 traits={ disinherited } }
  3={ first_name="Ermentrude" female=yes liege=1 }
  4={ first_name="Robert" liege=1 }
}
landed_titles={
  k_france={
    holder=1
    capital=c_paris
    government=feudal_government
    laws={ male_preference confederate_partition_succession_law crown_authority_1 }
    history={ huge=1 }
  }
  c_paris={ holder=1 }
  c_orleans={ holder=1 }
  d_valois={ holder=1 }
  c_blois={ holder=4 }
}
cultures={ french={ innovations={ gavelkind hereditary_rule } traditions={ } } }
wars={ 9={ casus_belli=claim_cb attacker=1 defender=8 warscore=10 } }
factions={ 3={ type=liberty_faction discontent=80 members={ 4 } } }
schemes={ 5={ type=murder target=2 owner=1 } }
armies={ 7={ owner=1 location=c_paris strength=1000 raiding=yes loot=50 } }
dynasties={ 10={ head=1 } }
"#;

    #[test]
    fn reads_the_sample_realm() {
        let mut collect = Collect::default();
        parse_text(SAMPLE, &mut collect).unwrap();
        assert_eq!(collect.fields.get("player_character").map(String::as_str), Some("1"));
        assert_eq!(collect.fields.get("version").map(String::as_str), Some("1.20.1"));
        assert_eq!(collect.people.get("1").unwrap().name, "Charles");
        assert_eq!(collect.people.get("1").unwrap().gold.as_deref(), Some("42.5"));
        assert!(collect.people.get("1").unwrap().traits.iter().any(|t| t == "brave"));
        let france = collect.titles.iter().find(|t| t.key == "k_france").unwrap();
        assert_eq!(france.holder.as_deref(), Some("1"));
        assert!(france.laws.iter().any(|l| l.contains("confederate")));
        assert!(!format!("{france:?}").contains("huge"));
        assert!(collect.wars.iter().any(|w| w.contains("attacker=1")));
        assert!(collect.armies.iter().any(|a| a.contains("raiding=yes")));
        assert!(collect.cultures.iter().any(|c| c.contains("gavelkind")));
    }
}
