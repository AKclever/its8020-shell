use crate::{
    expand,
    parser::{self, CommandSpec, Condition, Redirect, Word},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    process::{Child, Command, ExitStatus, Stdio},
};
#[derive(Default)]
pub struct Shell {
    pub exit: bool,
    pub status: i32,
    aliases: BTreeMap<String, Vec<String>>,
    variables: BTreeMap<String, String>,
    jobs: Vec<BackgroundJob>,
    next_job: u64,
}
struct BackgroundJob {
    id: u64,
    child: Child,
    name: String,
    finished: Option<ExitStatus>,
}
impl Shell {
    pub fn eval(&mut self, line: &str) {
        self.poll_jobs();
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
        while let Some(replacement) = self.aliases.get(&spec.words[0].text()) {
            if !seen.insert(spec.words[0].text()) || seen.len() > 16 {
                return Err("alias cycle or expansion limit".into());
            }
            spec.words
                .splice(0..1, replacement.iter().cloned().map(Word::literal));
        }
        let words = spec
            .words
            .iter()
            .map(|w| expand::expand_word(w, &self.variables, self.status))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let builtin = crate::editor::BUILTINS.contains(&words[0].as_str());
        let assignment = words.len() == 1 && words[0].contains('=');
        if spec.background && (builtin || assignment) {
            return Err("background execution is supported for external commands only".into());
        }
        let mut out_file = open_redirect(spec.stdout.as_ref(), &self.variables, self.status)?;
        let mut err_file = open_redirect(spec.stderr.as_ref(), &self.variables, self.status)?;
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
        if words.len() == 1 {
            if let Some((name, value)) = words[0].split_once('=') {
                if !expand::valid_name(name) {
                    return Err("invalid variable name".into());
                }
                self.variables.insert(name.to_owned(), value.to_owned());
                return Ok(0);
            }
        }
        match words[0].as_str() {
            "wait" if words.len() == 1 => self.wait_all(),
            "wait" => {
                writeln!(err, "usage: wait").map_err(|e| e.to_string())?;
                Ok(1)
            }
            "cd" | "ls" | "exit" | "alias" | "set" | "unset" | "jobs" => {
                match self.builtin(&words, out) {
                    Ok(()) => Ok(0),
                    Err(e) => {
                        writeln!(err, "{}: {e}", words[0]).map_err(|e| e.to_string())?;
                        Ok(1)
                    }
                }
            }
            program => {
                let mut command = Command::new(program);
                command.args(&words[1..]);
                command.envs(&self.variables);
                if let Some(f) = out_file {
                    command.stdout(Stdio::from(f));
                }
                if let Some(f) = &err_file {
                    command.stderr(Stdio::from(f.try_clone().map_err(|e| e.to_string())?));
                }
                if spec.background {
                    command.stdin(Stdio::null());
                }
                match command.spawn() {
                    Ok(child) if spec.background => {
                        self.next_job += 1;
                        eprintln!("[{}] pid {}", self.next_job, child.id());
                        self.jobs.push(BackgroundJob {
                            id: self.next_job,
                            child,
                            name: words.join(" "),
                            finished: None,
                        });
                        Ok(0)
                    }
                    Ok(mut child) => child
                        .wait()
                        .map(|s| s.code().unwrap_or(1))
                        .map_err(|e| e.to_string()),
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
            "jobs" if words.len() == 1 => {
                for job in &self.jobs {
                    let state = job
                        .finished
                        .map(|s| format!("Done({})", s.code().unwrap_or(1)))
                        .unwrap_or_else(|| "Running".into());
                    writeln!(
                        out,
                        "[{}] pid {} {state} {}",
                        job.id,
                        job.child.id(),
                        job.name
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            "jobs" => return Err("usage: jobs".into()),
            "set" if words.len() == 3 && expand::valid_name(&words[1]) => {
                self.variables.insert(words[1].clone(), words[2].clone());
            }
            "set" => return Err("usage: set NAME VALUE".into()),
            "unset" if words.len() == 2 && expand::valid_name(&words[1]) => {
                self.variables.remove(&words[1]);
            }
            "unset" => return Err("usage: unset NAME".into()),
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
                    if words[1].is_empty()
                        || !words[1]
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
                let path = words
                    .get(1)
                    .cloned()
                    .or_else(|| self.variables.get("HOME").cloned())
                    .map(Ok)
                    .unwrap_or_else(expand::home)?;
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
    fn poll_jobs(&mut self) {
        for job in &mut self.jobs {
            if job.finished.is_none() {
                match job.child.try_wait() {
                    Ok(Some(status)) => {
                        job.finished = Some(status);
                        eprintln!("[{}] done: {status}", job.id);
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("job {}: {e}", job.id),
                }
            }
        }
    }
    pub fn wait_all(&mut self) -> Result<i32, String> {
        let mut status = 0;
        // Keep any unprocessed handles if a wait fails, so shutdown can retry them.
        while !self.jobs.is_empty() {
            let job = &mut self.jobs[0];
            status = job
                .child
                .wait()
                .map_err(|e| e.to_string())?
                .code()
                .unwrap_or(1);
            self.jobs.remove(0);
        }
        Ok(status)
    }
}
fn open_redirect(
    redir: Option<&Redirect>,
    vars: &BTreeMap<String, String>,
    status: i32,
) -> Result<Option<File>, String> {
    redir
        .map(|r| {
            let paths = expand::expand_word(&r.path, vars, status)?;
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
