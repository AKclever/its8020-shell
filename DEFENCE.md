# Understanding and defending the shell

Read the earlier tags first. They show the program before the later features add
more structure. Use `git log --oneline --decorate` to see the progression.

## Where to read

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Choose terminal editor or piped input; repeatedly read and evaluate; save history and wait at shutdown |
| `src/parser.rs` | Turn characters into words/operators, retaining quote information; build commands and conditional links |
| `src/expand.rs` | Expand variables and paths; walk directories and match wildcard patterns |
| `src/shell.rs` | Own mutable state, dispatch built-ins, redirect output and manage child processes |
| `src/editor.rs` | Connect rustyline's helper traits to completion and highlighting |
| `tests/shell.rs` | Run the actual compiled shell as a child and check observable behavior |
| `tests/terminal_smoke.py` | Exercise the actual interactive terminal separately from piped tests |

## Six concepts to explain

1. **REPL and state.** The loop reads, evaluates and repeats. `Shell` owns variables,
   aliases, status and background handles. `cd` calls `set_current_dir` in this
   process; children inherit that directory. Rust's `Result` makes errors explicit,
   and the outer loop reports recoverable failures instead of calling `unwrap`.

2. **Processes and arguments.** `Command::new` chooses a program; `.args` gives it
   separate strings. A quoted space stays inside one string. We never construct a
   Bash command. Foreground `.spawn()` followed by `.wait()` blocks; a retained
   `Child` handle lets background work run without that immediate wait.

3. **Redirection and ownership.** `OpenOptions` opens a file in truncate or append
   mode. A built-in writes through `&mut dyn Write`, which can refer to stdout or a
   file. External processes receive owned file handles through `Stdio`. `try_clone`
   provides another handle where both parent diagnostics and child stderr need the
   file. This is OS-level output redirection, not collecting output after execution.

4. **Parsing before expansion.** The lexer preserves quote modes. The parser makes
   `CommandSpec`, `Redirect` and `Job` values. Only then does expansion produce
   argument strings. This order prevents a variable's semicolon from becoming a
   command separator. `NAME=Rust; echo $NAME` expands the second command after the
   first has changed state. A skipped conditional command is never expanded.

5. **Matching and directory traversal.** Each wildcard path component reads one
   directory. A dynamic-programming row tracks whether each prefix of a filename
   matches the pattern prefix. A literal character advances both prefixes. `*`
   either consumes no new character (`row[j]`) or one more (`next[j-1]`). Matching
   takes O(pattern length * filename length) time and O(filename length) memory.
   A private NUL marker protects literal stars during matching; actual NUL input
   is rejected, and the marker is decoded before producing paths. Traversal across
   many matching directories can still be expensive.

6. **Background lifecycle.** `spawn` starts a child, `try_wait` checks without
   blocking and reaps completed children, and `wait` blocks until termination.
   Keeping the `Child` matters: dropping a handle does not guarantee waiting for
   the process. Finished job records remain visible until `wait`; exit also waits.

## Trace an example aloud

For `NAME='Rust learner'; echo "$NAME" > greeting.txt`:

1. The lexer produces one assignment word, `;`, an `echo` word, a double-quoted
   variable word, `>`, and a filename word. Quotes are metadata, not argument bytes.
2. The parser builds two commands; the second has a stdout redirection.
3. The first command stores `NAME` in the session map and returns success.
4. The second expands `$NAME` to one argument containing a space.
5. The file opens in truncate mode; `echo` starts directly with that file as stdout.
6. The parent waits for the child and stores its exit status.

For `false && echo skipped || echo recovered`, `false` returns a nonzero status,
the first echo is skipped, and the second echo runs because the last executed
status is still nonzero. `&&` and `||` have equal precedence here.

## Questions to practise

- Why can't an external `cd` change the parent shell's working directory?
- What is the difference between a Rust `String`, a borrowed `&str`, and `&mut dyn Write`?
- What happens when a redirection file cannot be opened? Does the command run?
- How does `'*.rs'` differ from `*.rs`, and why preserve quotes until expansion?
- Why does this implementation use a map for variables and a set for alias cycles?
- What happens when a background process asks for input, finishes, or outlives `exit`?
- Which behavior is an official requirement and which behavior is our documented assumption?

Be candid about the limits in README: this is a small educational shell, with no
pipelines, full POSIX grammar or terminal job control. Understand a change before
presenting it in your defence.
