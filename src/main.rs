mod editor;
mod expand;
mod parser;
mod shell;
use rustyline::{error::ReadlineError, Editor};
use std::io::{self, IsTerminal};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut shell = shell::Shell::default();
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let history = expand::home()
        .ok()
        .map(|h| std::path::Path::new(&h).join(".its8020_shell_history"));
    let mut editor = if interactive {
        let mut ed = Editor::new()?;
        ed.set_helper(Some(editor::ShellHelper::new()));
        if let Some(path) = &history {
            let _ = ed.load_history(path);
        }
        Some(ed)
    } else {
        None
    };
    loop {
        let line = if let Some(ed) = &mut editor {
            let prompt = format!(
                "{} > ",
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "?".into())
            );
            match ed.readline(&prompt) {
                Ok(line) => {
                    let _ = ed.add_history_entry(line.as_str());
                    line
                }
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(e) => {
                    eprintln!("input: {e}");
                    break;
                }
            }
        } else {
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => line,
                Err(e) => {
                    eprintln!("input: {e}");
                    break;
                }
            }
        };
        shell.eval(&line);
        if shell.exit {
            break;
        }
    }
    if let Err(e) = shell.wait_all() {
        eprintln!("shutdown: {e}");
    }
    if let (Some(ed), Some(path)) = (&mut editor, history) {
        if let Err(e) = ed.save_history(&path) {
            eprintln!("history: {e}");
        }
    }
    Ok(())
}
