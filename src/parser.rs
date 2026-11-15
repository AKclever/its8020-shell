// Quote information survives parsing: expansion must distinguish '*' from "*".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quote {
    Bare,
    Double,
    Literal,
}
#[derive(Debug, Clone)]
pub struct Part {
    pub text: String,
    pub quote: Quote,
}
#[derive(Debug, Clone, Default)]
pub struct Word {
    pub parts: Vec<Part>,
}
impl Word {
    pub fn text(&self) -> String {
        self.parts.iter().map(|p| p.text.as_str()).collect()
    }
    pub fn literal(text: String) -> Self {
        Self {
            parts: vec![Part {
                text,
                quote: Quote::Literal,
            }],
        }
    }
    fn push(&mut self, c: char, quote: Quote) {
        if let Some(last) = self.parts.last_mut().filter(|p| p.quote == quote) {
            last.text.push(c);
        } else {
            self.parts.push(Part {
                text: c.to_string(),
                quote,
            });
        }
    }
}
#[derive(Debug, Clone)]
pub struct Redirect {
    pub path: Word,
    pub append: bool,
}
#[derive(Debug, Clone, Default)]
pub struct CommandSpec {
    pub background: bool,
    pub words: Vec<Word>,
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
#[derive(Debug)]
enum Token {
    Word(Word),
    Operator(String),
}

fn flush(word: &mut Word, tokens: &mut Vec<Token>) {
    if !word.parts.is_empty() {
        tokens.push(Token::Word(std::mem::take(word)));
    }
}

fn tokenize(line: &str) -> Result<Vec<Token>, String> {
    if line.contains('\0') {
        return Err("NUL is not allowed in input".into());
    }
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    let mut word = Word::default();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => flush(&mut word, &mut tokens),
            '#' if word.parts.is_empty() => break,
            '\\' => word.push(chars.next().ok_or("trailing escape")?, Quote::Literal),
            '\'' | '"' => {
                let quote = if c == '\'' {
                    Quote::Literal
                } else {
                    Quote::Double
                };
                // Keep empty quotes as a real (empty) argument.
                word.parts.push(Part {
                    text: String::new(),
                    quote,
                });
                loop {
                    let next = chars.next().ok_or("unclosed quote")?;
                    if next == c {
                        break;
                    }
                    if next == '\\' && c == '"' {
                        word.push(
                            chars.next().ok_or("trailing escape inside quote")?,
                            Quote::Literal,
                        );
                    } else {
                        word.push(next, quote);
                    }
                }
            }
            '>' | ';' | '&' | '|' => {
                let error_stream = c == '>'
                    && word.text() == "2"
                    && word.parts.iter().all(|p| p.quote == Quote::Bare);
                if error_stream {
                    word = Word::default();
                } else {
                    flush(&mut word, &mut tokens);
                }
                let mut op = if error_stream {
                    "2>".to_owned()
                } else {
                    c.to_string()
                };
                if c != ';' && chars.peek() == Some(&c) {
                    chars.next();
                    op.push(c);
                }
                if op == "|" {
                    return Err(format!("unsupported operator: {op}"));
                }
                tokens.push(Token::Operator(op));
            }
            '<' | '`' => return Err(format!("unsupported syntax: {c}")),
            '\0' => return Err("NUL is not allowed in input".into()),
            _ => word.push(c, Quote::Bare),
        }
    }
    flush(&mut word, &mut tokens);
    Ok(tokens)
}

pub fn parse(line: &str) -> Result<Vec<Job>, String> {
    let mut jobs = Vec::new();
    let mut spec = CommandSpec::default();
    let mut condition = Condition::Always;
    let mut tokens = tokenize(line)?.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            Token::Word(word) => spec.words.push(word),
            Token::Operator(op) if [">", ">>", "2>", "2>>"].contains(&op.as_str()) => {
                let Some(Token::Word(path)) = tokens.next() else {
                    return Err("expected redirection filename".into());
                };
                let slot = if op.starts_with('2') {
                    &mut spec.stderr
                } else {
                    &mut spec.stdout
                };
                if slot.is_some() {
                    return Err("duplicate stream redirection".into());
                }
                *slot = Some(Redirect {
                    path,
                    append: op.ends_with(">>"),
                });
            }
            Token::Operator(op) => {
                if spec.words.is_empty() {
                    return Err("missing command before operator".into());
                }
                if op == "&" {
                    if condition != Condition::Always {
                        return Err("background commands cannot be conditional".into());
                    }
                    spec.background = true;
                }
                jobs.push(Job {
                    command: std::mem::take(&mut spec),
                    condition,
                });
                condition = match op.as_str() {
                    "&&" => Condition::Success,
                    "||" => Condition::Failure,
                    _ => Condition::Always,
                };
                if tokens.peek().is_none() && op != ";" && op != "&" {
                    return Err("missing command after operator".into());
                }
            }
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
        assert_eq!(
            s.words.iter().map(Word::text).collect::<Vec<_>>(),
            ["echo", "hi"]
        );
        assert_eq!(s.stdout.unwrap().path.text(), "out");
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
            "echo 'unclosed",
            "echo \\",
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
    #[test]
    fn preserves_quoting_empty_words_and_literal_operators() {
        let jobs = parse("echo 'a b' \"\" a\\ b '>' x\"y z\" # comment").unwrap();
        assert_eq!(
            jobs[0]
                .command
                .words
                .iter()
                .map(Word::text)
                .collect::<Vec<_>>(),
            ["echo", "a b", "", "a b", ">", "xy z"]
        );
        assert_eq!(jobs[0].command.words[1].parts[0].quote, Quote::Literal);
    }
}
