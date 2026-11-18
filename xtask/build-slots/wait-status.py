import os
import signal
import subprocess
import sys
import time


class WaitStatus:
    def __init__(self, arguments):
        self.ignored = []
        self.group_signal = None
        self.ready_file = None
        self.delay = 0.0
        while arguments and arguments[0] != "--":
            option, value = arguments[0], arguments[1]
            arguments = arguments[2:]
            if option == "--ignore":
                self.ignored = [getattr(signal, "SIG" + name) for name in value.split(",")]
            elif option == "--group-signal":
                self.group_signal = getattr(signal, "SIG" + value)
            elif option == "--ready-file":
                self.ready_file = value
            elif option == "--delay":
                self.delay = float(value)
        self.command = arguments[1:]

    def ignore_signals(self):
        for number in self.ignored:
            signal.signal(number, signal.SIG_IGN)

    def wait_until_ready(self):
        deadline = time.monotonic() + 10
        while not os.path.exists(self.ready_file):
            if time.monotonic() > deadline:
                raise SystemExit("never ready")
            time.sleep(0.02)
        time.sleep(self.delay)

    def measure(self):
        if self.ready_file and os.path.exists(self.ready_file):
            os.unlink(self.ready_file)
        process = subprocess.Popen(self.command, start_new_session=True, preexec_fn=self.ignore_signals)
        if self.group_signal is not None:
            self.wait_until_ready()
            os.killpg(process.pid, self.group_signal)
        _, status = os.waitpid(process.pid, 0)
        if os.WIFSIGNALED(status):
            return "signal " + signal.Signals(os.WTERMSIG(status)).name
        return "exit %d" % os.WEXITSTATUS(status)


print(WaitStatus(sys.argv[1:]).measure())
