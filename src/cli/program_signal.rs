use std::io::{self, Write};
use std::process;

use super::exit_status::ExitStatus;
use crate::build::InterruptSignal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramSignal {
    number: i32,
}

impl ProgramSignal {
    const UNREPORTABLE_CODE: u8 = 1;

    #[cfg(unix)]
    pub fn of(status: process::ExitStatus) -> Option<Self> {
        use std::os::unix::process::ExitStatusExt;
        status.signal().map(|number| Self { number })
    }

    #[cfg(not(unix))]
    pub fn of(_status: process::ExitStatus) -> Option<Self> {
        None
    }

    pub fn is_interrupt(self) -> bool {
        self.number == i32::from(InterruptSignal::NUMBER)
    }

    pub fn shell_code(self) -> u8 {
        u8::try_from(i32::from(ExitStatus::SIGNAL_BASE) + self.number)
            .unwrap_or(Self::UNREPORTABLE_CODE)
    }

    pub fn kill_this_process(self) {
        let _unreportable = io::stdout().flush();
        self.raise_with_default_action();
    }

    #[cfg(unix)]
    fn raise_with_default_action(self) {
        Self::forbid_core_files();
        self.restore_default_action();
        self.unblock();
        unsafe { libc::raise(self.number) };
    }

    #[cfg(unix)]
    fn restore_default_action(self) {
        unsafe {
            let mut default_action: libc::sigaction = std::mem::zeroed();
            default_action.sa_sigaction = libc::SIG_DFL;
            libc::sigemptyset(&mut default_action.sa_mask);
            libc::sigaction(self.number, &default_action, std::ptr::null_mut());
        }
    }

    #[cfg(unix)]
    fn unblock(self) {
        unsafe {
            let mut only_this_signal: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut only_this_signal);
            libc::sigaddset(&mut only_this_signal, self.number);
            libc::pthread_sigmask(libc::SIG_UNBLOCK, &only_this_signal, std::ptr::null_mut());
        }
    }

    #[cfg(unix)]
    fn forbid_core_files() {
        let no_core_file = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        unsafe { libc::setrlimit(libc::RLIMIT_CORE, &no_core_file) };
    }

    #[cfg(not(unix))]
    fn raise_with_default_action(self) {}
}
