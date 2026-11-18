use super::primitive::PrimitiveType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Conversion {
    Index,
    OptionalIndex,
}

impl Conversion {
    pub const ARGUMENT: PrimitiveType = PrimitiveType::I64;
    const MARKERS: [(&'static str, Conversion); 2] = [
        ("index ", Conversion::Index),
        ("index? ", Conversion::OptionalIndex),
    ];
    const WRITTEN_OPENING: &'static str = "{index";

    pub fn may_be_written_in(template: &str) -> bool {
        template.contains(Self::WRITTEN_OPENING)
    }

    pub fn split(written: &str) -> (Option<Self>, &str) {
        Self::MARKERS
            .iter()
            .find_map(|(marker, conversion)| {
                written
                    .strip_prefix(marker)
                    .map(|rest| (Some(*conversion), rest))
            })
            .unwrap_or((None, written))
    }

    pub fn placeholder(self, position: usize) -> String {
        format!("{{{}{position}}}", self.marker())
    }

    pub fn rejects_negative_literal(self) -> bool {
        matches!(self, Self::Index)
    }

    pub fn yields_option(self) -> bool {
        matches!(self, Self::OptionalIndex)
    }

    pub fn leaves_argument_type_open(self) -> bool {
        matches!(self, Self::OptionalIndex)
    }

    fn marker(self) -> &'static str {
        Self::MARKERS
            .iter()
            .find(|(_, conversion)| *conversion == self)
            .map(|(marker, _)| *marker)
            .expect("every conversion has a marker")
    }
}

#[cfg(test)]
mod tests {
    use super::Conversion;

    #[test]
    fn splits_each_marker_from_its_placeholder() {
        assert_eq!(Conversion::split("index 1"), (Some(Conversion::Index), "1"));
        assert_eq!(
            Conversion::split("index? 1"),
            (Some(Conversion::OptionalIndex), "1")
        );
        assert_eq!(Conversion::split("1"), (None, "1"));
    }

    #[test]
    fn finds_a_conversion_only_in_a_template_that_may_write_one() {
        assert!(Conversion::may_be_written_in("&{0}[{index 1}]"));
        assert!(Conversion::may_be_written_in("{0}.get({index? 1})"));
        assert!(!Conversion::may_be_written_in("&{0}[{1}]"));
    }

    #[test]
    fn writes_each_placeholder_with_its_marker() {
        assert_eq!(Conversion::Index.placeholder(0), "{index 0}");
        assert_eq!(Conversion::OptionalIndex.placeholder(2), "{index? 2}");
    }

    #[test]
    fn rejects_a_negative_literal_only_where_it_panics() {
        assert!(Conversion::Index.rejects_negative_literal());
        assert!(!Conversion::OptionalIndex.rejects_negative_literal());
    }

    #[test]
    fn yields_an_option_and_leaves_its_argument_open_only_when_optional() {
        assert!(Conversion::OptionalIndex.yields_option());
        assert!(!Conversion::Index.yields_option());
        assert!(Conversion::OptionalIndex.leaves_argument_type_open());
        assert!(!Conversion::Index.leaves_argument_type_open());
    }

    #[test]
    fn writes_the_marker_it_splits() {
        for conversion in [Conversion::Index, Conversion::OptionalIndex] {
            let written = format!("{}0", conversion.marker());
            assert_eq!(Conversion::split(&written), (Some(conversion), "0"));
        }
    }
}
