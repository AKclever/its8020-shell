use std::env;
use std::io::{self, Write};
mod expand;

fn main() {
    loop {
        print!(
            "{} > ",
            env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "?".into())
        );
        if io::stdout().flush().is_err() {
            break;
        }
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("input: {e}");
                break;
            }
        }
        let words = match line
            .split_whitespace()
            .map(expand::expand)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(words) => words.into_iter().flatten().collect::<Vec<_>>(),
            Err(e) => {
                eprintln!("expansion: {e}");
                continue;
            }
        };
        let Some(command) = words.first() else {
            continue;
        };
        let result: Result<(), String> = match command.as_str() {
            "exit" if words.len() == 1 => break,
            "exit" => Err("usage: exit".into()),
            "cd" if words.len() <= 2 => {
                let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"));
                match words.get(1).map(|s| s.to_string()).or_else(|| home.ok()) {
                    Some(path) => env::set_current_dir(path).map_err(|e| e.to_string()),
                    None => Err("HOME is not set".into()),
                }
            }
            "cd" => Err("usage: cd [directory]".into()),
            "ls" if words.len() <= 2 => (|| {
                let entries = std::fs::read_dir(words.get(1).map(String::as_str).unwrap_or("."))
                    .map_err(|e| e.to_string())?;
                let mut names = entries
                    .map(|e| e.map(|e| e.file_name()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                names.sort();
                for name in names {
                    println!("{}", name.to_string_lossy());
                }
                Ok(())
            })(),
            "ls" => Err("usage: ls [directory]".into()),
            _ => std::process::Command::new(command)
                .args(&words[1..])
                .status()
                .map(|status| {
                    if !status.success() {
                        eprintln!("{command}: {status}");
                    }
                })
                .map_err(|e| e.to_string()),
        };
        if let Err(e) = result {
            eprintln!("{command}: {e}");
        }
    }
}
