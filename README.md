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

## Milestone 2

External commands use `Command::new(program).args(arguments).status()` directly.
The child inherits input/output and the shell waits for it. PATH resolves executable
names. No Bash, PowerShell or cmd interpreter is involved.
Bare `~` and a leading `~/` expand to HOME; bare `.` and leading `./` expand to the
current directory. `*` matches zero or more characters within a path component;
multiple wildcard components work. Matches are sorted and case-sensitive.
Hidden names need a pattern starting with a dot. An unmatched pattern stays literal.
No `?`, character classes, recursive `**`, or `~otheruser` expansion is supported.
Try `pwd`, `echo hello Rust`, `ls ~`, `echo .`, `echo src/*.rs`, `false`, and an invalid command.

## Milestone 3

In a terminal, rustyline provides line editing, Up/Down history, and Tab completion.
History is saved to `$HOME/.its8020_shell_history`; it may contain sensitive input,
so avoid typing credentials. Built-in command names are highlighted green.
Completion offers built-ins and executable PATH entries for the first word, and
filenames for arguments. Completion is deliberately basic.
Noninteractive input uses the same executor without prompts, history or colors.
`>` and `>>` redirect/append stdout; `2>` and `2>>` redirect/append stderr.
These work for built-ins and external programs. Redirection targets must expand to
one path. Duplicate redirections for the same stream are rejected. Opening a file
can create/truncate it even when starting the command subsequently fails.
File redirection is implemented; process-to-process `|` pipelines are outside the
published wording and are not supported. Quoting is deferred to milestone 5.
Try `echo hello > greeting.txt`, `cat greeting.txt`, `cd /missing 2> errors.txt`,
and `cat errors.txt`. These commands create files in the current directory.

## Milestone 4

`a ; b` always runs b, `a && b` runs b after success, and `a || b` runs b after
failure. Conditions are evaluated left-to-right with equal precedence for `&&`
and `||`; `;` starts an unconditional command. Nonzero means failure, including
failed built-ins. A trailing `;` is accepted, a trailing `&&` or `||` is an error.
The whole line is parsed before any commands execute.
`alias greet echo hello` defines an in-memory command-word substitution;
`greet Rust` runs `echo hello Rust`. `alias` lists aliases; `alias greet` shows one.
Aliases may chain but cycles and expansion depth over 16 are errors. Definitions
are not saved between sessions and cannot contain operators. This explicit syntax
is a project choice; POSIX `alias name=value` is not supported.
Try `false && echo skipped; false || echo recovered` and the alias example above.

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
