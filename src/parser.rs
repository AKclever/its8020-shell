#[derive(Debug, Clone)]
pub struct Redirect {
    pub path: String,
    pub append: bool,
}
#[derive(Debug, Clone, Default)]
pub struct CommandSpec {
    pub words: Vec<String>,
    pub stdout: Option<Redirect>,
    pub stderr: Option<Redirect>,
}

pub fn parse(line: &str) -> Result<Option<CommandSpec>, String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut word = String::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == '>' {
            let error_stream = c == '>' && word == "2";
            if !word.is_empty() && !error_stream {
                tokens.push(std::mem::take(&mut word));
            }
            word.clear();
            if c == '>' {
                let mut op = if error_stream { "2>" } else { ">" }.to_owned();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    op.push('>');
                }
                tokens.push(op);
            }
        } else if "|&;<".contains(c) {
            return Err(format!("unsupported operator: {c}"));
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    let mut spec = CommandSpec::default();
    let mut tokens = tokens.into_iter();
    while let Some(token) = tokens.next() {
        if [">", ">>", "2>", "2>>"].contains(&token.as_str()) {
            let path = tokens.next().ok_or("missing redirection filename")?;
            if path.contains('>') {
                return Err("expected redirection filename".into());
            }
            let slot = if token.starts_with('2') {
                &mut spec.stderr
            } else {
                &mut spec.stdout
            };
            if slot.is_some() {
                return Err("duplicate stream redirection".into());
            }
            *slot = Some(Redirect {
                path,
                append: token.ends_with(">>"),
            });
        } else {
            spec.words.push(token);
        }
    }
    if spec.words.is_empty() {
        if spec.stdout.is_some() || spec.stderr.is_some() {
            return Err("missing command".into());
        }
        Ok(None)
    } else {
        Ok(Some(spec))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirections_and_errors() {
        let s = parse("echo hi>out 2>>err").unwrap().unwrap();
        assert_eq!(s.words, ["echo", "hi"]);
        assert_eq!(s.stdout.unwrap().path, "out");
        assert!(s.stderr.unwrap().append);
        for text in ["ls >", "ls > > x", "> x", "ls > a > b"] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
}
