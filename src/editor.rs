use rustyline::{
    completion::{Completer, FilenameCompleter, Pair},
    highlight::{CmdKind, Highlighter},
    hint::Hinter,
    validate::Validator,
    Context, Helper,
};
use std::{borrow::Cow, collections::BTreeSet, env, fs};
pub const BUILTINS: &[&str] = &["cd", "ls", "exit", "alias", "set", "unset"];
pub struct ShellHelper {
    files: FilenameCompleter,
}
impl ShellHelper {
    pub fn new() -> Self {
        Self {
            files: FilenameCompleter::new(),
        }
    }
}
impl Helper for ShellHelper {}
impl Hinter for ShellHelper {
    type Hint = String;
}
impl Validator for ShellHelper {}
impl Completer for ShellHelper {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos].rfind(char::is_whitespace).map_or(0, |i| i + 1);
        let prefix = &line[start..pos];
        if start != 0 || prefix.contains('/') {
            return self.files.complete(line, pos, ctx);
        }
        let mut names: BTreeSet<String> = BUILTINS.iter().map(|s| s.to_string()).collect();
        if let Some(path) = env::var_os("PATH") {
            for dir in env::split_paths(&path) {
                if let Ok(entries) = fs::read_dir(dir) {
                    for e in entries.flatten() {
                        if is_executable(&e.path()) {
                            names.insert(e.file_name().to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
        Ok((
            start,
            names
                .into_iter()
                .filter(|n| n.starts_with(prefix))
                .map(|n| Pair {
                    display: n.clone(),
                    replacement: n,
                })
                .collect(),
        ))
    }
}
fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}
impl Highlighter for ShellHelper {
    fn highlight<'l>(&self, line: &'l str, _: usize) -> Cow<'l, str> {
        let leading = line.len() - line.trim_start().len();
        let end = line[leading..]
            .find(char::is_whitespace)
            .map_or(line.len(), |i| leading + i);
        if BUILTINS.contains(&&line[leading..end]) {
            Cow::Owned(format!(
                "{}\x1b[1;32m{}\x1b[0m{}",
                &line[..leading],
                &line[leading..end],
                &line[end..]
            ))
        } else {
            Cow::Borrowed(line)
        }
    }
    fn highlight_char(&self, _: &str, _: usize, _: CmdKind) -> bool {
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn highlights_builtins_without_changing_text() {
        let h = ShellHelper::new();
        assert_eq!(h.highlight("  cd src", 4), "  \x1b[1;32mcd\x1b[0m src");
        assert_eq!(h.highlight("echo hello", 0), "echo hello");
    }
    #[test]
    fn completes_commands_and_files() {
        let h = ShellHelper::new();
        let history = rustyline::history::DefaultHistory::new();
        let ctx = Context::new(&history);
        assert!(h
            .complete("exi", 3, &ctx)
            .unwrap()
            .1
            .iter()
            .any(|p| p.replacement == "exit"));
        assert!(h
            .complete("ls Cargo.t", 10, &ctx)
            .unwrap()
            .1
            .iter()
            .any(|p| p.replacement == "Cargo.toml"));
    }
}
