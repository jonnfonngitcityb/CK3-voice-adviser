mod binaryparse;
mod model;
mod render;
mod textparse;
mod tokens;

use crate::binaryparse::parse_binary;
use crate::model::Collect;
use crate::render::render;
use crate::textparse::parse_text;
use crate::tokens::load_tokens;
use jomini::envelope::{JominiFile, SaveContentKind, SaveMetadataKind};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8, String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "probe" => {
            let path = rest
                .first()
                .ok_or("usage: ck3-extract probe <save>")?;
            let (ironman, version) = probe_file(Path::new(path))?;
            println!("ironman={ironman}");
            println!("version={version}");
            Ok(0)
        }
        "write" => cmd_write(&rest),
        _ => Err("usage: ck3-extract probe <save> | write <save> --md <path> --json <path> --label <name>".to_string()),
    }
}

fn cmd_write(args: &[String]) -> Result<u8, String> {
    let save = args
        .first()
        .ok_or("usage: ck3-extract write <save> --md <path> --json <path> --label <name>")?;
    let md = flag(args, "--md")?;
    let json = flag(args, "--json")?;
    let label = flag(args, "--label").unwrap_or_else(|_| {
        Path::new(save)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| save.clone())
    });
    let tokens = load_tokens();
    eprintln!("tokens={}", tokens.source);
    let bytes = fs::read(save).map_err(|err| format!("read {save}: {err}"))?;
    let loaded = load_bytes(&bytes, &tokens.map);
    eprintln!("header={}", loaded.header_note);
    let (markdown, json_text, header_only) = render(&loaded.collect, &label, loaded.binary_failed);
    write_output(Path::new(&md), &markdown)?;
    write_output(Path::new(&json), &json_text)?;
    let keys = loaded
        .collect
        .top_keys
        .iter()
        .take(40)
        .cloned()
        .collect::<Vec<_>>()
        .join(",");
    eprintln!(
        "decode={} people={} titles={} unknown_tokens={} resolved_binary_keys={}",
        if header_only { "header-only" } else { "gamestate" },
        loaded.collect.people.len(),
        loaded.collect.titles.len(),
        loaded.collect.unknown_tokens,
        loaded.collect.resolved_binary_keys
    );
    eprintln!("keys={keys}");
    if header_only {
        Ok(2)
    } else {
        Ok(0)
    }
}

struct Loaded {
    collect: Collect,
    binary_failed: bool,
    header_note: String,
}

fn load_bytes(bytes: &[u8], tokens: &std::collections::HashMap<u16, String>) -> Loaded {
    let mut collect = Collect::default();
    let mut binary_failed = false;
    let header_note;
    let mut game_binary = false;
    match JominiFile::from_slice(bytes) {
        Ok(save) => {
            header_note = format!("{:?}", save.header().kind());
            if let Ok((buf, binary)) = read_meta(&save) {
                let numbers = collect.binary_numbers;
                feed(&buf, binary, false, tokens, &mut collect, &mut binary_failed);
                collect.binary_numbers = numbers;
            }
            if let Ok((buf, binary)) = read_game(&save) {
                game_binary = binary;
                let resolved_before = collect.resolved_binary_keys;
                let unknown_before = collect.unknown_tokens;
                feed(&buf, binary, true, tokens, &mut collect, &mut binary_failed);
                if binary && !tokens.is_empty() {
                    let added_resolved = collect.resolved_binary_keys - resolved_before;
                    let added_unknown = collect.unknown_tokens - unknown_before;
                    if added_resolved == 0 && added_unknown > 0 {
                        clear_realm(&mut collect);
                        binary_failed = true;
                    }
                }
            } else {
                binary_failed = true;
            }
        }
        Err(err) => {
            header_note = format!("not-an-envelope ({err})");
            if looks_like_text(bytes) {
                let text = String::from_utf8_lossy(bytes);
                if let Err(parse_err) = parse_text(&text, &mut collect) {
                    eprintln!("text parse: {parse_err}");
                    binary_failed = collect.people.is_empty() && collect.titles.is_empty();
                }
            } else if !tokens.is_empty() {
                game_binary = true;
                if let Err(parse_err) = parse_binary(bytes, tokens, &mut collect) {
                    eprintln!("binary parse stopped: {parse_err}");
                    if collect.people.is_empty() && collect.titles.is_empty() {
                        binary_failed = true;
                    }
                }
                if collect.resolved_binary_keys == 0 && collect.unknown_tokens > 0 {
                    clear_realm(&mut collect);
                    binary_failed = true;
                }
            } else {
                binary_failed = true;
            }
        }
    }
    scrape_fields(&bytes[..bytes.len().min(2 * 1024 * 1024)], &mut collect);
    if binary_failed {
        collect.binary_numbers = false;
    } else if game_binary && (!collect.people.is_empty() || !collect.titles.is_empty()) {
        collect.binary_numbers = true;
    }
    let body = !collect.people.is_empty() || !collect.titles.is_empty();
    if !body && (binary_failed || game_binary && tokens.is_empty()) {
        binary_failed = true;
    }
    Loaded {
        collect,
        binary_failed,
        header_note,
    }
}

fn read_meta<R: jomini::envelope::ReaderAt>(
    save: &JominiFile<R>,
) -> Result<(Vec<u8>, bool), String> {
    let mut meta = save.meta().map_err(|err| err.to_string())?;
    let binary = matches!(&meta, SaveMetadataKind::Binary(_));
    let mut buf = Vec::new();
    meta.read_to_end(&mut buf).map_err(|err| err.to_string())?;
    Ok((buf, binary))
}

fn read_game<R: jomini::envelope::ReaderAt>(
    save: &JominiFile<R>,
) -> Result<(Vec<u8>, bool), String> {
    let mut game = save.gamestate().map_err(|err| err.to_string())?;
    let binary = matches!(&game, SaveContentKind::Binary(_));
    let mut buf = Vec::new();
    game.read_to_end(&mut buf).map_err(|err| err.to_string())?;
    Ok((buf, binary))
}

fn feed(
    buf: &[u8],
    binary: bool,
    is_game: bool,
    tokens: &std::collections::HashMap<u16, String>,
    collect: &mut Collect,
    binary_failed: &mut bool,
) {
    if buf.is_empty() {
        if is_game {
            *binary_failed = true;
        }
        return;
    }
    if binary {
        if tokens.is_empty() {
            if is_game {
                *binary_failed = true;
            }
            scrape_fields(buf, collect);
            return;
        }
        if let Err(err) = parse_binary(buf, tokens, collect) {
            eprintln!("binary parse stopped: {err}");
            if is_game && collect.people.is_empty() && collect.titles.is_empty() {
                *binary_failed = true;
            }
        }
        return;
    }
    let text = String::from_utf8_lossy(buf);
    if let Err(err) = parse_text(&text, collect) {
        eprintln!("text parse: {err}");
        if is_game && collect.people.is_empty() && collect.titles.is_empty() {
            *binary_failed = true;
        }
    }
}

fn clear_realm(collect: &mut Collect) {
    collect.people.clear();
    collect.titles.clear();
    collect.wars.clear();
    collect.factions.clear();
    collect.schemes.clear();
    collect.armies.clear();
    collect.cultures.clear();
    collect.faiths.clear();
    collect.dynasties.clear();
}

fn probe_file(path: &Path) -> Result<(String, String), String> {
    let file = File::open(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut buf = Vec::new();
    match JominiFile::from_file(file) {
        Ok(save) => {
            eprintln!("header={:?}", save.header().kind());
            if let Ok(mut meta) = save.meta() {
                let binary = matches!(&meta, SaveMetadataKind::Binary(_));
                let mut raw = Vec::new();
                meta.read_to_end(&mut raw).map_err(|err| err.to_string())?;
                if !binary && looks_like_text(&raw) {
                    let text = String::from_utf8_lossy(&raw);
                    let mut collect = Collect::default();
                    let _ = parse_text(&text, &mut collect);
                    scrape_fields(&raw, &mut collect);
                    return Ok((ironman_label(&collect), version_label(&collect)));
                }
                buf = raw;
            }
        }
        Err(err) => {
            eprintln!("header=unreadable ({err})");
            let mut file = File::open(path).map_err(|err| err.to_string())?;
            let mut limited = vec![0u8; 1024 * 1024];
            let n = file.read(&mut limited).map_err(|err| err.to_string())?;
            buf = limited[..n].to_vec();
            if looks_like_text(&buf) {
                let text = String::from_utf8_lossy(&buf);
                let mut collect = Collect::default();
                let _ = parse_text(&text, &mut collect);
                scrape_fields(&buf, &mut collect);
                return Ok((ironman_label(&collect), version_label(&collect)));
            }
        }
    }
    let mut collect = Collect::default();
    scrape_fields(&buf, &mut collect);
    Ok((ironman_label(&collect), version_label(&collect)))
}

fn ironman_label(collect: &Collect) -> String {
    match collect.fields.get("ironman").map(String::as_str) {
        Some("yes") | Some("true") => "yes".to_string(),
        Some("no") | Some("false") => "no".to_string(),
        _ => "unknown".to_string(),
    }
}

fn version_label(collect: &Collect) -> String {
    collect
        .fields
        .get("version")
        .or_else(|| collect.fields.get("save_game_version"))
        .cloned()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn scrape_fields(bytes: &[u8], collect: &mut Collect) {
    let window = &bytes[..bytes.len().min(2 * 1024 * 1024)];
    if collect.fields.get("version").is_none() && collect.fields.get("save_game_version").is_none()
    {
        if let Some(version) = find_version(window) {
            collect.set_field("version", version);
        }
    }
    if collect.fields.get("ironman").is_none() {
        if let Some(ironman) = find_ironman(window) {
            collect.set_field("ironman", ironman);
        }
    }
}

fn find_version(bytes: &[u8]) -> Option<String> {
    let key = b"version";
    let mut index = 0;
    while index + key.len() < bytes.len() {
        if bytes[index..].starts_with(key) {
            let mut cursor = index + key.len();
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= bytes.len() || bytes[cursor] != b'=' {
                index += 1;
                continue;
            }
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'"' {
                cursor += 1;
            }
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor].is_ascii_digit() || bytes[cursor] == b'.')
            {
                cursor += 1;
            }
            if cursor > start && bytes[start..cursor].contains(&b'.') {
                return Some(String::from_utf8_lossy(&bytes[start..cursor]).into_owned());
            }
        }
        index += 1;
    }
    None
}

fn find_ironman(bytes: &[u8]) -> Option<String> {
    if contains_ignore_ascii(bytes, b"ironman=\"yes\"") || contains_ignore_ascii(bytes, b"ironman=yes")
    {
        return Some("yes".to_string());
    }
    if contains_ignore_ascii(bytes, b"ironman=\"no\"") || contains_ignore_ascii(bytes, b"ironman=no")
    {
        return Some("no".to_string());
    }
    None
}

fn contains_ignore_ascii(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || hay.len() < needle.len() {
        return false;
    }
    hay.windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn looks_like_text(bytes: &[u8]) -> bool {
    let sample = &bytes[..bytes.len().min(400)];
    if sample.is_empty() || !sample.contains(&b'=') {
        return false;
    }
    let printable = sample
        .iter()
        .filter(|byte| byte.is_ascii_graphic() || byte.is_ascii_whitespace())
        .count();
    printable * 5 >= sample.len() * 4
}

fn write_output(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
    }
    let mut file = File::create(path).map_err(|err| format!("create {}: {err}", path.display()))?;
    file.write_all(text.as_bytes())
        .map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(())
}

fn flag(args: &[String], name: &str) -> Result<String, String> {
    let position = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("missing {name}"))?;
    args.get(position + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_save_becomes_gamestate() {
        let text = br#"
date=867.1.1
player_character=1
ironman=yes
version="1.20.1"
living={
  1={ first_name="Charles" liege=0 }
}
landed_titles={
  k_france={ holder=1 government=feudal_government }
}
"#;
        let loaded = load_bytes(text, &std::collections::HashMap::new());
        assert!(!loaded.binary_failed);
        let (markdown, _, header_only) = render(&loaded.collect, "sample.ck3", loaded.binary_failed);
        assert!(!header_only);
        assert!(markdown.contains("Decode: gamestate"));
        assert!(markdown.contains("Charles"));
        assert!(markdown.contains("1.20.1"));
    }
}
