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

#[cfg(unix)]
#[test]
fn chaining_short_circuits_and_aliases_are_bounded() {
    let output = run("false && echo BAD; true || echo BAD; false || echo recovered\nalias greet echo hello\ngreet Rust\nalias a b\nalias b a\na\necho after-cycle\nexit; echo BAD\n");
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(!out.contains("BAD"));
    assert!(out.contains("recovered"));
    assert!(out.contains("hello Rust"));
    assert!(out.contains("after-cycle"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("alias cycle"));
}

#[cfg(unix)]
#[test]
fn quoting_variables_and_status_remain_data() {
    let output = run("ITS8020_TEST_NAME='hello world'\nprintf '<%s>\\n' \"$ITS8020_TEST_NAME\" '$ITS8020_TEST_NAME' \"\" a\\ b\necho '*'\nfalse; echo status=$?\nset MESSAGE 'a; echo INJECTED'\nprintf '<%s>\\n' $MESSAGE\nprintenv ITS8020_TEST_NAME\nunset ITS8020_TEST_NAME\necho ${ITS8020_TEST_NAME}end\necho 'unterminated\necho after-error\nexit\n");
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.contains("<hello world>\n<$ITS8020_TEST_NAME>\n<>\n<a b>"),
        "{out}"
    );
    assert!(out.contains("\n*\n"));
    assert!(out.contains("status=1"));
    assert!(out.contains("<a; echo INJECTED>"));
    assert!(!out.lines().any(|l| l == "INJECTED"));
    assert!(out.contains("\nhello world\n"));
    assert!(out.contains("\nend\n"));
    assert!(out.contains("after-error"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unclosed quote"));
}

#[cfg(unix)]
#[test]
fn quoted_paths_work_and_syntax_errors_have_no_side_effects() {
    let dir = std::env::temp_dir().join(format!("its8020 quoted {}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("output file");
    let output = run(&format!(
        "echo BAD > '{}' &&\necho good > '{}'\ncat '{}'\nexit\n",
        file.display(),
        file.display(),
        file.display()
    ));
    assert_eq!(fs::read_to_string(&file).unwrap(), "good\n");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "good\n");
    fs::remove_dir_all(dir).unwrap();
}
