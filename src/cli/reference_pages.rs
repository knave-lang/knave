use super::doc_folder::WrittenPage;

macro_rules! reference_folder {
    () => {
        "docs/reference"
    };
}

macro_rules! index_source {
    () => {
        "README"
    };
}

macro_rules! reference_source {
    ($name:expr) => {
        include_str!(concat!("../../", reference_folder!(), "/", $name, ".md"))
    };
}

pub struct ReferencePages;

impl ReferencePages {
    pub const FOLDER: &'static str = reference_folder!();
    pub const INDEX_SOURCE: &'static str = index_source!();
    pub const WRITTEN_FOLDER: &'static str = "reference";
    const PAGES: [(&'static str, &'static str); 2] = [
        (index_source!(), reference_source!(index_source!())),
        ("bindings", reference_source!("bindings")),
    ];

    pub fn pages() -> Vec<WrittenPage> {
        Self::PAGES
            .iter()
            .map(|(source, text)| WrittenPage {
                module: Self::written_module(source),
                text: text.to_string(),
            })
            .collect()
    }

    pub fn written_file(source: &str) -> String {
        WrittenPage::file_name(&Self::written_module(source))
    }

    fn written_module(source: &str) -> String {
        if source == Self::INDEX_SOURCE {
            Self::WRITTEN_FOLDER.to_string()
        } else {
            format!("{}/{source}", Self::WRITTEN_FOLDER)
        }
    }
}
