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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
    Always,
    Success,
    Failure,
}
#[derive(Debug)]
pub struct Job {
    pub command: CommandSpec,
    pub condition: Condition,
}

fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut word = String::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || ">;&|".contains(c) {
            let error_stream = c == '>' && word == "2";
            if !word.is_empty() && !error_stream {
                tokens.push(std::mem::take(&mut word));
            }
            word.clear();
            if !c.is_whitespace() {
                let mut op = if error_stream {
                    "2>".to_owned()
                } else {
                    c.to_string()
                };
                if c != ';' && chars.peek() == Some(&c) {
                    chars.next();
                    op.push(c);
                }
                if op == "|" || op == "&" {
                    return Err(format!("unsupported operator: {op}"));
                }
                tokens.push(op);
            }
        } else if c == '<' {
            return Err("input redirection is not supported".into());
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    Ok(tokens)
}

pub fn parse(line: &str) -> Result<Vec<Job>, String> {
    let mut jobs = Vec::new();
    let mut spec = CommandSpec::default();
    let mut condition = Condition::Always;
    let mut tokens = tokenize(line)?.into_iter().peekable();
    while let Some(token) = tokens.next() {
        if [">", ">>", "2>", "2>>"].contains(&token.as_str()) {
            let path = tokens.next().ok_or("missing redirection filename")?;
            if [">", ">>", "2>", "2>>", ";", "&&", "||"].contains(&path.as_str()) {
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
        } else if [";", "&&", "||"].contains(&token.as_str()) {
            if spec.words.is_empty() {
                return Err("missing command before operator".into());
            }
            jobs.push(Job {
                command: std::mem::take(&mut spec),
                condition,
            });
            condition = match token.as_str() {
                "&&" => Condition::Success,
                "||" => Condition::Failure,
                _ => Condition::Always,
            };
            if tokens.peek().is_none() && token != ";" {
                return Err("missing command after operator".into());
            }
        } else {
            spec.words.push(token);
        }
    }
    if !spec.words.is_empty() {
        jobs.push(Job {
            command: spec,
            condition,
        });
    } else if spec.stdout.is_some() || spec.stderr.is_some() {
        return Err("missing command".into());
    }
    Ok(jobs)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirections_and_errors() {
        let mut jobs = parse("echo hi>out 2>>err").unwrap();
        let s = jobs.remove(0).command;
        assert_eq!(s.words, ["echo", "hi"]);
        assert_eq!(s.stdout.unwrap().path, "out");
        assert!(s.stderr.unwrap().append);
        for text in [
            "ls >",
            "ls > > x",
            "> x",
            "ls > a > b",
            "echo hi &&",
            "; ls",
            "ls ;; ls",
            "ls | cat",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn parses_chain_conditions() {
        let jobs = parse("true && echo yes || echo no; ls;").unwrap();
        assert_eq!(
            jobs.iter().map(|j| j.condition).collect::<Vec<_>>(),
            [
                Condition::Always,
                Condition::Success,
                Condition::Failure,
                Condition::Always
            ]
        );
    }
}
