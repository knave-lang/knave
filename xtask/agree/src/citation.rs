use crate::finding_signature::FindingSignature;
use crate::known_classes::KnownClasses;
use crate::verdict::Rejection;

#[derive(Clone, Debug)]
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

    pub fn signature(&self, oracle: &'static str) -> FindingSignature {
        FindingSignature::new(
            oracle,
            self.rejections.iter().map(Rejection::place).collect(),
        )
    }

    pub fn is_explained_by(&self, classes: &KnownClasses) -> bool {
        self.rejections
            .iter()
            .all(|rejection| classes.know(rejection.code.as_ref()))
    }
}
