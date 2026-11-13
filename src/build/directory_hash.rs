use std::fmt;
use std::path::{self, Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryHash {
    digits: String,
}

impl DirectoryHash {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    const SHORT_HASH_MASK: u64 = 0xffff_ffff;

    pub fn of(directory: &Path) -> Self {
        let hash = Self::absolute_before_it_exists(directory)
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .fold(Self::FNV_OFFSET, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(Self::FNV_PRIME)
            });
        Self {
            digits: format!("{:08x}", hash & Self::SHORT_HASH_MASK),
        }
    }

    fn absolute_before_it_exists(directory: &Path) -> PathBuf {
        path::absolute(directory).unwrap_or_else(|_| directory.to_path_buf())
    }
}

impl fmt::Display for DirectoryHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.digits)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;

    use super::DirectoryHash;

    #[test]
    fn two_directories_hash_apart() {
        assert_ne!(
            DirectoryHash::of(Path::new("/work/target/knave/first")),
            DirectoryHash::of(Path::new("/work/target/knave/second"))
        );
    }

    #[test]
    fn a_relative_directory_hashes_as_its_path_from_the_working_directory() {
        let relative = Path::new("target/knave/first");
        let working_directory = env::current_dir().expect("the working directory resolves");
        assert_eq!(
            DirectoryHash::of(relative),
            DirectoryHash::of(&working_directory.join(relative))
        );
    }

    #[test]
    fn a_directory_that_does_not_exist_yet_hashes_to_eight_hex_digits() {
        let digits = DirectoryHash::of(Path::new("/nowhere/target/knave/first")).to_string();
        assert_eq!(digits.len(), 8);
        assert!(digits.chars().all(|digit| digit.is_ascii_hexdigit()));
    }
}
