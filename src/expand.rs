use crate::parser::{Quote, Word};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Component, Path, PathBuf},
};

pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn variables(text: &str, vars: &BTreeMap<String, String>, status: i32) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            result.push(c);
            continue;
        }
        let value = match chars.peek() {
            Some('?') => {
                chars.next();
                status.to_string()
            }
            Some('(') => return Err("command substitution is not supported".into()),
            Some('{') => {
                chars.next();
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => name.push(c),
                        None => return Err("unclosed variable reference".into()),
                    }
                }
                if !valid_name(&name) {
                    return Err("invalid variable name".into());
                }
                vars.get(&name)
                    .cloned()
                    .or_else(|| env::var(&name).ok())
                    .unwrap_or_default()
            }
            Some(c) if c.is_ascii_alphabetic() || *c == '_' => {
                let mut name = String::new();
                while chars
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    name.push(chars.next().unwrap());
                }
                vars.get(&name)
                    .cloned()
                    .or_else(|| env::var(&name).ok())
                    .unwrap_or_default()
            }
            _ => {
                result.push('$');
                continue;
            }
        };
        // Expanded values remain data: no re-parsing, word splitting or new glob operators.
        result.push_str(&value.replace('*', "\0"));
    }
    Ok(result)
}

pub fn expand_word(
    word: &Word,
    vars: &BTreeMap<String, String>,
    status: i32,
) -> Result<Vec<String>, String> {
    let mut text = String::new();
    for part in &word.parts {
        let value = if part.quote == Quote::Literal {
            part.text.clone()
        } else {
            variables(&part.text, vars, status)?
        };
        if part.quote == Quote::Bare {
            text.push_str(&value);
        } else {
            text.push_str(&value.replace('*', "\0"));
        }
    }
    let path_prefix = word.parts.first().is_some_and(|p| {
        p.quote == Quote::Bare && (p.text.starts_with('~') || p.text.starts_with('.'))
    });
    if path_prefix && (text == "~" || text.starts_with("~/")) {
        let home = vars
            .get("HOME")
            .cloned()
            .map(Ok)
            .unwrap_or_else(home)?
            .replace('*', "\0");
        text = format!("{home}{}", &text[1..]);
    }
    expand_pattern(&text, path_prefix)
}

pub fn home() -> Result<String, String> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| "HOME is not set".into())
}

// '*' matches any number of characters; matching never crosses a directory separator.
pub fn matches(pattern: &str, name: &str) -> bool {
    let p: Vec<_> = pattern.chars().collect();
    let n: Vec<_> = name.chars().collect();
    let mut row = vec![false; n.len() + 1];
    row[0] = true;
    for c in p {
        let mut next = vec![false; n.len() + 1];
        if c == '*' {
            next[0] = row[0];
        }
        for j in 1..=n.len() {
            next[j] = if c == '*' {
                row[j] || next[j - 1]
            } else {
                row[j - 1] && (if c == '\0' { '*' } else { c }) == n[j - 1]
            };
        }
        row = next;
    }
    row[n.len()]
}

#[cfg(test)]
pub fn expand(word: &str) -> Result<Vec<String>, String> {
    expand_pattern(word, true)
}

fn expand_pattern(word: &str, path_prefix: bool) -> Result<Vec<String>, String> {
    let word = if path_prefix && word == "~" {
        home()?
    } else if let Some(rest) = word.strip_prefix("~/").filter(|_| path_prefix) {
        format!("{}/{rest}", home()?)
    } else {
        word.to_owned()
    };
    let word = if path_prefix && word == "." {
        env::current_dir()
            .map_err(|e| e.to_string())?
            .display()
            .to_string()
    } else if let Some(rest) = word.strip_prefix("./").filter(|_| path_prefix) {
        env::current_dir()
            .map_err(|e| e.to_string())?
            .join(rest)
            .display()
            .to_string()
    } else {
        word
    };
    if !word.contains('*') {
        return Ok(vec![word.replace('\0', "*")]);
    }
    let mut paths = vec![PathBuf::new()];
    for component in Path::new(&word).components() {
        match component {
            Component::Normal(part) if part.to_string_lossy().contains('*') => {
                let pattern = part.to_string_lossy();
                let mut next = Vec::new();
                for base in paths {
                    let dir = if base.as_os_str().is_empty() {
                        Path::new(".")
                    } else {
                        &base
                    };
                    let entries = match fs::read_dir(dir) {
                        Ok(entries) => entries,
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                            ) =>
                        {
                            continue
                        }
                        Err(e) => return Err(e.to_string()),
                    };
                    for entry in entries {
                        let entry = entry.map_err(|e| e.to_string())?;
                        let name = entry.file_name();
                        let name_text = name.to_string_lossy();
                        if (!name_text.starts_with('.') || pattern.starts_with('.'))
                            && matches(&pattern, &name_text)
                        {
                            next.push(base.join(name));
                        }
                    }
                }
                paths = next;
            }
            _ => {
                for path in &mut paths {
                    path.push(component.as_os_str().to_string_lossy().replace('\0', "*"));
                }
            }
        }
    }
    let mut results: Vec<_> = paths
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| p.display().to_string())
        .collect();
    results.sort();
    if results.is_empty() {
        results.push(word.replace('\0', "*"));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_matching() {
        for (p, n, expected) in [
            ("*", "", true),
            ("a*b*c", "abZZc", true),
            ("*.rs", "main.rs", true),
            ("a*b", "abc", false),
            ("é*", "été", true),
        ] {
            assert_eq!(matches(p, n), expected, "{p} {n}");
        }
    }
    #[test]
    fn expands_real_entries_and_preserves_unmatched() {
        let base = env::temp_dir().join(format!("its8020-expand-{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("one.rs"), "").unwrap();
        fs::write(base.join("two.txt"), "").unwrap();
        assert_eq!(
            expand(&format!("{}/*.rs", base.display())).unwrap(),
            vec![base.join("one.rs").display().to_string()]
        );
        let missing = format!("{}/*.absent", base.display());
        assert_eq!(expand(&missing).unwrap(), vec![missing]);
        fs::remove_dir_all(base).unwrap();
    }
}
