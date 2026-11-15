use crate::{
    expand,
    parser::{self, CommandSpec, Condition, Redirect},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    process::{Command, Stdio},
};
#[derive(Default)]
pub struct Shell {
    pub exit: bool,
    pub status: i32,
    aliases: BTreeMap<String, Vec<String>>,
}
impl Shell {
    pub fn eval(&mut self, line: &str) {
        match parser::parse(line) {
            Ok(jobs) => {
                for job in jobs {
                    if self.exit {
                        break;
                    }
                    if (job.condition == Condition::Success && self.status != 0)
                        || (job.condition == Condition::Failure && self.status == 0)
                    {
                        continue;
                    }
                    self.status = self.execute(job.command).unwrap_or_else(|e| {
                        eprintln!("shell: {e}");
                        1
                    });
                }
            }
            Err(e) => {
                eprintln!("syntax: {e}");
                self.status = 2;
            }
        }
    }
    fn execute(&mut self, mut spec: CommandSpec) -> Result<i32, String> {
        // Substitute command words only. Alias text cannot inject control operators.
        let mut seen = BTreeSet::new();
        while let Some(replacement) = self.aliases.get(&spec.words[0]) {
            if !seen.insert(spec.words[0].clone()) || seen.len() > 16 {
                return Err("alias cycle or expansion limit".into());
            }
            spec.words.splice(0..1, replacement.clone());
        }
        let words = spec
            .words
            .iter()
            .map(|w| expand::expand(w))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let mut out_file = open_redirect(spec.stdout.as_ref())?;
        let mut err_file = open_redirect(spec.stderr.as_ref())?;
        let mut stdout = io::stdout();
        let mut stderr = io::stderr();
        let out: &mut dyn Write = match &mut out_file {
            Some(f) => f,
            None => &mut stdout,
        };
        let err: &mut dyn Write = match &mut err_file {
            Some(f) => f,
            None => &mut stderr,
        };
        match words[0].as_str() {
            "cd" | "ls" | "exit" | "alias" => match self.builtin(&words, out) {
                Ok(()) => Ok(0),
                Err(e) => {
                    writeln!(err, "{}: {e}", words[0]).map_err(|e| e.to_string())?;
                    Ok(1)
                }
            },
            program => {
                let mut command = Command::new(program);
                command.args(&words[1..]);
                if let Some(f) = out_file {
                    command.stdout(Stdio::from(f));
                }
                if let Some(f) = &err_file {
                    command.stderr(Stdio::from(f.try_clone().map_err(|e| e.to_string())?));
                }
                match command.status() {
                    Ok(status) => Ok(status.code().unwrap_or(1)),
                    Err(e) => {
                        let err: &mut dyn Write = match &mut err_file {
                            Some(f) => f,
                            None => &mut stderr,
                        };
                        writeln!(err, "{program}: {e}").map_err(|e| e.to_string())?;
                        Ok(127)
                    }
                }
            }
        }
    }
    fn builtin(&mut self, words: &[String], out: &mut dyn Write) -> Result<(), String> {
        match words[0].as_str() {
            "alias" => match words.len() {
                1 => {
                    for (name, value) in &self.aliases {
                        writeln!(out, "{name}: {}", value.join(" ")).map_err(|e| e.to_string())?;
                    }
                }
                2 => {
                    let value = self.aliases.get(&words[1]).ok_or("alias not found")?;
                    writeln!(out, "{}: {}", words[1], value.join(" "))
                        .map_err(|e| e.to_string())?;
                }
                _ => {
                    if !words[1]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        return Err("alias name must contain letters, digits or underscore".into());
                    }
                    self.aliases.insert(words[1].clone(), words[2..].to_vec());
                }
            },
            "exit" if words.len() == 1 => self.exit = true,
            "exit" => return Err("usage: exit".into()),
            "cd" if words.len() <= 2 => {
                let path = words.get(1).cloned().map(Ok).unwrap_or_else(expand::home)?;
                env::set_current_dir(path).map_err(|e| e.to_string())?;
            }
            "cd" => return Err("usage: cd [directory]".into()),
            "ls" if words.len() <= 2 => {
                let mut entries = fs::read_dir(words.get(1).map(String::as_str).unwrap_or("."))
                    .map_err(|e| e.to_string())?
                    .map(|e| e.map(|e| e.file_name()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                entries.sort();
                for name in entries {
                    writeln!(out, "{}", name.to_string_lossy()).map_err(|e| e.to_string())?;
                }
            }
            "ls" => return Err("usage: ls [directory]".into()),
            _ => unreachable!(),
        }
        Ok(())
    }
}
fn open_redirect(redir: Option<&Redirect>) -> Result<Option<File>, String> {
    redir
        .map(|r| {
            let paths = expand::expand(&r.path)?;
            if paths.len() != 1 {
                return Err("redirection needs exactly one filename".into());
            }
            OpenOptions::new()
                .write(true)
                .create(true)
                .append(r.append)
                .truncate(!r.append)
                .open(&paths[0])
                .map_err(|e| format!("{}: {e}", paths[0]))
        })
        .transpose()
}
