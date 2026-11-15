# ITS8020 shell

An educational Rust shell for TalTech ITS8020, autumn 2026.
Authoritative requirements: <https://github.com/kilpkonn/its8020-materials/blob/master/hw.md>
(read on 15 November 2026). Syntax choices below are project assumptions, not additional course requirements.

## Setup and run

Preferred environment: Ubuntu in WSL with VS Code. Open this directory in a WSL terminal,
then run `cargo run`. Run `cargo test` for automated checks and `cargo fmt --check`
for formatting. Rust is installed with rustup; compilation also needs Ubuntu's
`build-essential` package. No interpreter is used to execute user commands.

## Milestone 1

The loop prints and flushes a prompt, reads one line, separates words, dispatches
a command, and repeats. Empty lines do nothing; EOF exits. Ordinary errors print
to stderr and return to the prompt. `cd` uses `std::env::set_current_dir` in this
process: changing a child process's directory would not change the shell's directory.
`ls` uses `read_dir`, sorts names, and prints one entry per line.

Try `ls`, `cd ..`, `ls`, `cd ~` (tilde expansion arrives in milestone 2), and `exit`.
At this stage `cd` without an argument uses HOME (USERPROFILE fallback on Windows).
`ls [directory]` includes hidden entries. Options such as `ls -l` are not implemented.
`exit` takes no arguments. Words are separated by whitespace; quoting arrives later.

## Submission questions

The homework must be defended before the exam. A GitHub push does not establish
submission. Do not upload this project to the earlier C Assignment 01 area.
Ask Tavo Annus:

- Which milestones must be ready for the Moodle reservation labelled "week 5"?
- Should a repository link or files be sent, and where?
- How are missed intermediate deadlines handled?
- Are the documented choices for chaining, variables, quoting, wildcards and background jobs acceptable?

Only dependencies explicitly allowed by the homework may be used; ask Tavo before
adding another. Milestones 1 and 2 use only the standard library, including tests.
