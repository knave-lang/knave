use std::io::{self, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum Completion {
    Finished(Finished),
    TimedOut,
}

pub struct Finished {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub struct TimedCommand {
    command: Command,
    stdin: Vec<u8>,
}

impl TimedCommand {
    const POLL_INTERVAL: Duration = Duration::from_millis(5);

    pub fn new(command: Command) -> Self {
        Self {
            command,
            stdin: Vec::new(),
        }
    }

    pub fn with_stdin(self, stdin: Vec<u8>) -> Self {
        Self { stdin, ..self }
    }

    pub fn run(mut self, limit: Duration) -> io::Result<Completion> {
        let mut child = self
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let streams = ChildStreams::attach(&mut child, self.stdin);
        match Self::wait(&mut child, limit)? {
            Some(status) => Ok(Completion::Finished(streams.finish(status))),
            None => Self::stop(child, streams),
        }
    }

    fn wait(child: &mut Child, limit: Duration) -> io::Result<Option<ExitStatus>> {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if let Some(status) = child.try_wait()? {
                return Ok(Some(status));
            }
            thread::sleep(Self::POLL_INTERVAL);
        }
        Ok(None)
    }

    fn stop(mut child: Child, streams: ChildStreams) -> io::Result<Completion> {
        child.kill()?;
        let status = child.wait()?;
        streams.finish(status);
        Ok(Completion::TimedOut)
    }
}

struct ChildStreams {
    stdout: JoinHandle<Vec<u8>>,
    stderr: JoinHandle<Vec<u8>>,
}

impl ChildStreams {
    fn attach(child: &mut Child, stdin: Vec<u8>) -> Self {
        if let Some(mut input) = child.stdin.take() {
            thread::spawn(move || input.write_all(&stdin));
        }
        Self {
            stdout: Self::drain(child.stdout.take()),
            stderr: Self::drain(child.stderr.take()),
        }
    }

    fn drain(stream: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut stream) = stream {
                let _ = stream.read_to_end(&mut bytes);
            }
            bytes
        })
    }

    fn finish(self, status: ExitStatus) -> Finished {
        Finished {
            status,
            stdout: self.stdout.join().unwrap_or_default(),
            stderr: self.stderr.join().unwrap_or_default(),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::{Completion, TimedCommand};

    struct CommandFixture;

    impl CommandFixture {
        const SHORT_LIMIT: Duration = Duration::from_millis(100);
        const LONG_LIMIT: Duration = Duration::from_secs(30);

        fn shell(script: &str) -> TimedCommand {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            TimedCommand::new(command)
        }
    }

    #[test]
    fn stops_a_command_past_its_limit() {
        let started = Instant::now();
        let completion = CommandFixture::shell("sleep 30").run(CommandFixture::SHORT_LIMIT);
        assert!(matches!(completion, Ok(Completion::TimedOut)));
        assert!(started.elapsed() < CommandFixture::LONG_LIMIT);
    }

    #[test]
    fn hands_standard_input_over_and_keeps_both_outputs() {
        let completion = CommandFixture::shell("cat; echo note >&2")
            .with_stdin(b"typed".to_vec())
            .run(CommandFixture::LONG_LIMIT);
        let Ok(Completion::Finished(finished)) = completion else {
            panic!("the command finishes");
        };
        assert_eq!(finished.stdout, b"typed");
        assert_eq!(finished.stderr, b"note\n");
    }
}
