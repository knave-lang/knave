use super::announced_write::AnnouncedWrite;
use super::exit_status::ExitStatus;

pub struct PendingWrites {
    reads: Vec<Result<Option<AnnouncedWrite>, ExitStatus>>,
}

impl PendingWrites {
    pub fn into_writes(self) -> Result<Vec<AnnouncedWrite>, ExitStatus> {
        let reads: Vec<Option<AnnouncedWrite>> =
            self.reads.into_iter().collect::<Result<_, _>>()?;
        Ok(reads.into_iter().flatten().collect())
    }
}

impl FromIterator<Result<Option<AnnouncedWrite>, ExitStatus>> for PendingWrites {
    fn from_iter<T: IntoIterator<Item = Result<Option<AnnouncedWrite>, ExitStatus>>>(
        reads: T,
    ) -> Self {
        Self {
            reads: reads.into_iter().collect(),
        }
    }
}

impl FromIterator<PendingWrites> for PendingWrites {
    fn from_iter<T: IntoIterator<Item = PendingWrites>>(components: T) -> Self {
        Self {
            reads: components
                .into_iter()
                .flat_map(|pending| pending.reads)
                .collect(),
        }
    }
}
