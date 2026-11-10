use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

pub struct CrateLock {
    _file: File,
}

impl CrateLock {
    const FILE_NAME: &'static str = "build.lock";

    pub fn acquire(directory: &Path, waiting: impl FnOnce()) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(directory.join(Self::FILE_NAME))?;
        if !Self::try_lock(&file)? {
            waiting();
            Self::lock(&file)?;
        }
        Ok(Self { _file: file })
    }
}

#[cfg(unix)]
impl CrateLock {
    fn try_lock(file: &File) -> io::Result<bool> {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::WouldBlock => Ok(false),
            _ => Err(error),
        }
    }

    fn lock(file: &File) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(());
        }
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
impl CrateLock {
    fn try_lock(_file: &File) -> io::Result<bool> {
        Ok(true)
    }

    fn lock(_file: &File) -> io::Result<()> {
        Ok(())
    }
}
