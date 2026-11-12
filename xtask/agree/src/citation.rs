use crate::verdict::Rejection;

pub struct Citation {
    judgement: &'static str,
    rejections: Vec<Rejection>,
}

impl Citation {
    pub fn new(judgement: &'static str, rejections: Vec<Rejection>) -> Self {
        Self {
            judgement,
            rejections,
        }
    }

    pub fn summary(&self) -> String {
        let cited: Vec<String> = self.rejections.iter().map(Rejection::to_string).collect();
        format!("{}: {}", self.judgement, cited.join(", "))
    }

    pub fn into_rejections(self) -> Vec<Rejection> {
        self.rejections
    }
}
