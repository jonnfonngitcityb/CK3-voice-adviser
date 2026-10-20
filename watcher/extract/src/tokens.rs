use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TokenTable {
    pub map: HashMap<u16, String>,
    pub source: String,
}

pub fn load_tokens() -> TokenTable {
    if let Some(path) = std::env::var_os("CK3_IRONMAN_TOKENS") {
        let path = PathBuf::from(path);
        return read_table(&path, "CK3_IRONMAN_TOKENS");
    }
    if let Some(path) = cache_path() {
        if path.is_file() {
            let table = read_table(&path, "cache");
            if !table.map.is_empty() {
                return table;
            }
        }
    }
    if let Some(path) = find_oos_dump() {
        if let Some(cache) = cache_path() {
            if !cache.is_file() {
                if let Some(parent) = cache.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&path, &cache).is_ok() {
                    eprintln!("token cache copied to {}", cache.display());
                }
            }
        }
        return read_table(&path, "oos_dump");
    }
    TokenTable {
        map: HashMap::new(),
        source: "missing".to_string(),
    }
}

pub fn parse_token_lines(text: &str) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        if let Some((id, name)) = parse_token_line(line) {
            map.insert(id, name);
        }
    }
    map
}

fn parse_token_line(line: &str) -> Option<(u16, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    let second = parts.next()?;
    if let Some(id) = parse_id(first) {
        return Some((id, second.to_string()));
    }
    if let Some(id) = parse_id(second) {
        return Some((id, first.to_string()));
    }
    None
}

fn parse_id(text: &str) -> Option<u16> {
    if let Some(hex) = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16).ok();
    }
    if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) {
        return text.parse().ok();
    }
    None
}

fn read_table(path: &Path, label: &str) -> TokenTable {
    match fs::read_to_string(path) {
        Ok(text) => {
            let map = parse_token_lines(&text);
            let source = format!("{label}:{}", path.display());
            if map.is_empty() {
                eprintln!("token file had no ids: {}", path.display());
            }
            TokenTable { map, source }
        }
        Err(err) => {
            eprintln!("token file: {err}");
            TokenTable {
                map: HashMap::new(),
                source: format!("unreadable:{label}:{}", path.display()),
            }
        }
    }
}

fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(base)
            .join("ck3-guide")
            .join("tokens.txt"),
    )
}

fn find_oos_dump() -> Option<PathBuf> {
    let profile = PathBuf::from(std::env::var_os("USERPROFILE")?);
    let mut roots = vec![
        profile.join("Documents").join(oos_tail()),
        profile.join("OneDrive").join("Documents").join(oos_tail()),
    ];
    if let Ok(entries) = fs::read_dir(&profile) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("OneDrive") {
                roots.push(entry.path().join("Documents").join(oos_tail()));
            }
        }
    }
    for root in roots {
        let preferred = root.join("dummy").join("tokens_1.tok");
        if preferred.is_file() {
            return Some(preferred);
        }
        if let Some(found) = find_named(&root) {
            return Some(found);
        }
    }
    None
}

fn oos_tail() -> PathBuf {
    PathBuf::from("Paradox Interactive")
        .join("Crusader Kings III")
        .join("oos")
}

fn find_named(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut depth = 0;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < 2 {
                    stack.push(path);
                }
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("tokens") && name.ends_with(".tok") {
                return Some(path);
            }
        }
        depth += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_token_line_orders() {
        let map = parse_token_lines(
            "# comment\n\n0xffff player_character\nfirst_name 42\n0x0010 landed_titles\n",
        );
        assert_eq!(map.get(&0xffff).map(String::as_str), Some("player_character"));
        assert_eq!(map.get(&42).map(String::as_str), Some("first_name"));
        assert_eq!(map.get(&16).map(String::as_str), Some("landed_titles"));
    }
}
