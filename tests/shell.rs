use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

fn run(input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_its8020-shell"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn repl_recovers_from_errors_and_handles_eof() {
    let output = run("\ncommand_that_does_not_exist_its8020\nls\n");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("command_that_does_not_exist_its8020"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Cargo.toml"));
}

#[test]
fn cd_persists_and_exit_stops_processing() {
    let dir = std::env::temp_dir().join(format!("its8020-cd-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("persistent-directory-marker"), "").unwrap();
    let output = run(&format!(
        "cd {}\nls\nexit\nmissing_after_exit\n",
        dir.display()
    ));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("persistent-directory-marker"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("missing_after_exit"));
    fs::remove_dir_all(dir).unwrap();
}
