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

#[cfg(unix)]
#[test]
fn external_arguments_expansion_and_failure_recovery() {
    let output = run("/usr/bin/printf %s:%s first second\necho src/*.rs\necho ~\necho .\nfalse\necho recovered\nexit\n");
    assert!(output.status.success());
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(out.contains("first:second"));
    assert!(out.contains("src/main.rs"));
    assert!(out.contains(&std::env::var("HOME").unwrap()));
    assert!(out.contains(&std::env::current_dir().unwrap().display().to_string()));
    assert!(out.contains("recovered"));
}

#[cfg(unix)]
#[test]
fn redirects_both_streams_for_programs_and_builtins() {
    let dir = std::env::temp_dir().join(format!("its8020-redir-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let out = dir.join("out");
    let err = dir.join("err");
    let listing = dir.join("listing");
    let output = run(&format!("echo first > {}\necho second >> {}\ncd /missing-its8020-directory 2> {}\nmissing_its8020_program 2>> {}\nls src > {}\nexit\n", out.display(), out.display(), err.display(), err.display(), listing.display()));
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(out).unwrap(), "first\nsecond\n");
    let errors = fs::read_to_string(err).unwrap();
    assert!(errors.contains("cd:"));
    assert!(errors.contains("missing_its8020_program"));
    assert!(fs::read_to_string(listing).unwrap().contains("main.rs"));
    fs::remove_dir_all(dir).unwrap();
}
