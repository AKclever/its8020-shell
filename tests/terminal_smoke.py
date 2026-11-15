"""Optional Linux/WSL terminal checks. Run after cargo build; no Python packages needed."""
import os
import pty
import select
import signal
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/debug/its8020-shell"


class Terminal:
    def __init__(self, home):
        self.pending = b""
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(ROOT)
            os.environ.update(HOME=str(home), TERM="xterm")
            os.execv(BINARY, [str(BINARY)])
        self.read_until(b" > ")

    def send(self, data):
        os.write(self.fd, data)

    def read_until(self, marker):
        data = self.pending
        self.pending = b""
        deadline = time.monotonic() + 10
        while marker not in data:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise AssertionError((marker, data))
            if select.select([self.fd], [], [], remaining)[0]:
                data += os.read(self.fd, 65536)
        end = data.index(marker) + len(marker)
        self.pending = data[end:]
        return data[:end]

    def close(self):
        self.send(b"exit\n")
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                os.close(self.fd)
                assert os.waitstatus_to_exitcode(status) == 0
                return
            time.sleep(0.02)
        os.kill(self.pid, signal.SIGKILL)
        os.waitpid(self.pid, 0)
        raise AssertionError("shell did not exit")


with tempfile.TemporaryDirectory(prefix="its8020-terminal-") as home:
    terminal = Terminal(home)
    terminal.send(b"cd sr\t")
    colored = terminal.read_until(b"src/")
    assert b"\x1b[1;32m" in colored, colored
    terminal.send(b"\n")
    terminal.read_until(b"\r\n")
    terminal.read_until(b"/src > ")
    terminal.send(b"cd ..\n")
    terminal.read_until(b"\r\n")
    terminal.read_until(b" > ")
    terminal.send(b"echo HISTORY_MARKER\n")
    terminal.read_until(b"\r\nHISTORY_MARKER\r\n")
    terminal.read_until(b" > ")
    terminal.send(b"\x1b[A\n")  # Up recalls the preceding command.
    recalled = terminal.read_until(b"\r\nHISTORY_MARKER\r\n")
    assert b"HISTORY_MARKER\r\n" in recalled, recalled
    terminal.read_until(b" > ")
    terminal.close()
    assert "echo HISTORY_MARKER" in Path(home, ".its8020_shell_history").read_text()

    terminal = Terminal(home)
    terminal.send(b"\x1b[A\x1b[A\n")  # Skip saved 'exit'; recall the earlier command.
    recalled = terminal.read_until(b"\r\nHISTORY_MARKER\r\n")
    assert b"HISTORY_MARKER\r\n" in recalled, recalled
    terminal.read_until(b" > ")
    terminal.close()

print("Terminal smoke passed: file completion, color, history recall and persistence")
