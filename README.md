# ITS8020 shell

An educational Rust shell for TalTech ITS8020, autumn 2026.
Authoritative requirements: <https://github.com/kilpkonn/its8020-materials/blob/master/hw.md>
(read on 15 November 2026). Syntax choices below are project assumptions, not additional course requirements.

## Setup and run

Preferred environment: Ubuntu in WSL with VS Code. Open this directory in a WSL terminal,
then run `cargo run`. Run `cargo test` for automated checks and `cargo fmt --check`
for formatting. Rust is installed with rustup; compilation also needs Ubuntu's
`build-essential` package. No interpreter is used to execute user commands.

On this Windows computer, open PowerShell and run:

```powershell
wsl -d Ubuntu -u aleks
```

Then, inside Ubuntu:

```sh
cd '/mnt/c/Users/aleks/sys programming/its8020-shell'
source ~/.cargo/env
cargo run
```

For another computer: install Ubuntu/WSL, `sudo apt update`,
`sudo apt install build-essential git curl`, and follow <https://rust-lang.org/tools/install>.
Clone your private repository, enter the directory, and run `cargo run --locked`.
Windows-native Rust was not installed or tested; Ubuntu is the tested target.
VS Code's WSL extension is installed. Use **WSL: Connect to WSL using Distro**,
select Ubuntu, then open this directory. No password was set for the new Linux
user; administrative setup can use `wsl -d Ubuntu -u root` from PowerShell.

## Status and learning checkpoints

| Stage | Published deadline | Status | Direct dependencies |
| --- | --- | --- | --- |
| 1 | Course week 10 | Implemented and tested; tag `milestone-1` | None |
| 2 | Course week 11 | Implemented and tested; tag `milestone-2` | None |
| 3 | Course week 12 | Implemented and tested; tag `milestone-3` | rustyline |
| 4 | Course week 14 | Implemented and tested; tag `milestone-4` | rustyline |
| 5 | Course week 15 | Implemented and tested; tag `milestone-5` | rustyline |
| 6 | Before exam | Implemented and tested; tag `milestone-6` | rustyline |

Each tag is a runnable point in the same project's history. To study an earlier
stage without changing your working directory, use `git worktree add ../shell-stage-1 milestone-1`,
then run Cargo in that directory. Do not run later-stage demo commands in earlier stages.
Cargo.lock records rustyline's transitive dependencies; no additional direct or
development dependencies are used. The whitelist is interpreted to allow the
normal dependencies of rustyline; confirm with Tavo if the course means otherwise.

See [DEFENCE.md](DEFENCE.md) for the source map, Rust concepts and defence questions.

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

## Milestone 5

The hand-written lexer preserves quote information in `Word`/`Part` values;
the parser produces command and redirection structures before execution. `pest`
is allowed but not required: this small grammar uses the standard library.
Single quotes preserve literal text. Double quotes allow variables but suppress
wildcards and tilde expansion. A backslash escapes the next character, including
inside double quotes (a deliberate simplification). Adjacent quoted/unquoted
parts form one argument; empty quotes produce an empty argument. A `#` at a word
boundary starts a comment. Unclosed quotes and trailing escapes are errors.

`NAME=value` as a standalone command or `set NAME VALUE` sets a session variable.
Names follow `[A-Za-z_][A-Za-z0-9_]*`. `$NAME`, `${NAME}`, and `$?` expand; `$?`
is the previous executed command's numeric status. Undefined variables become
empty strings. `unset NAME` removes the session override (an inherited environment
value becomes visible again). Session variables are passed to child environments;
they do not change the parent terminal and are not saved. Command-prefix assignments
such as `NAME=value program` are not supported. Alias values capture expanded
arguments when defined; invocation appends new arguments.

Variable values stay in one argument and do not introduce new wildcard or control
operators. Only unquoted source `*` performs globbing; quoted or escaped `*` is
literal, including mixed patterns. Bare source `~`, `~/`, `.`, and `./` perform path
expansion; variable contents do not trigger another path expansion. Expansion is
performed just before each executed command, so `NAME=Rust; echo $NAME` works.
No command substitution, arithmetic, subshells, multiline syntax or full POSIX
compatibility is claimed. Use WSL paths with `/`; quote paths containing spaces.
Input and filenames are handled as UTF-8 text; non-UTF-8 Unix filenames may be
displayed lossily. `exit` and EOF end successfully; command statuses are used by
`$?` and chaining, rather than being the shell executable's final exit code.
Try `NAME='hello Rust'; echo "$NAME"`, `echo '$NAME'`, `echo "*"`, and
`false; echo $?`.

## Milestone 6

`sleep 2 &` launches an external process without waiting. `jobs` lists its ID, PID
and Running/Done state. `wait` waits for all jobs, removes their records and returns
the last job's exit status (zero when there are none). Children are polled with
`try_wait` before each input line is evaluated; completion is reported at that
point, rather than asynchronously while editing. Finished records stay until `wait`.
Background stdin is disconnected so a child cannot consume shell commands; stdout
and stderr remain inherited unless redirected. Use `program >log.txt 2>err.txt &`
to avoid output interrupting your prompt.

Background built-ins and assignments are rejected because they mutate shell state.
Background commands cannot be members of conditional chains (`true && sleep 2 &`
is rejected); `sleep 2 & echo ready` is supported. These are explicit scope choices.
On `exit` or EOF the shell waits for all outstanding jobs, so long-running jobs can
delay shutdown. Stop long-running programs externally if needed. There is no `fg`,
`bg`, terminal process-group management, signal forwarding or interactive job control.
Ctrl-C at an editable prompt cancels the line; Ctrl-C during a foreground process
may terminate this simple shell too. This is a remaining job-control limitation.
Try `sleep 2 &`, `jobs`, `echo responsive`, `wait`, and `jobs`.

## Verification

Run in Ubuntu from the project directory:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
python3 tests/terminal_smoke.py
```

The 18 Rust tests cover real child processes, persistent directory changes,
errors/EOF/exit, wildcard matching, both redirected streams, chaining, alias cycles,
quoting, variables, malformed input, and nonblocking background execution.
The optional Python standard-library terminal test verifies actual Tab filename
completion, ANSI highlighting, Up-arrow recall and history across restarts, using
a temporary HOME. No extra Python packages or Cargo development crates are needed.
Tests use small temporary directories and remove them on success. Linux external
commands in integration tests are available in Ubuntu. No TalTech grading or
lecturer acceptance is implied by these checks.

## GitHub

Private repository: <https://github.com/AKclever/its8020-shell>.
`main` and the six milestone tags hold the staged project. `target/`, credentials,
editor settings and local environment files are ignored. Do not commit generated
demo output. Existing Windows Git credentials were used without adding tokens to
project files. Windows Git is configured; Linux Git may need your identity and
authentication if you choose to push from WSL.

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
