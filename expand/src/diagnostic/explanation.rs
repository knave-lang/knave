use super::DiagnosticCode;

macro_rules! written_explanations {
    ($($code:literal,)*) => {
        impl Explanation {
            const WRITTEN: &[(&str, &str)] =
                &[$(($code, include_str!(concat!("explanations/", $code, ".md")))),*];
        }
    };
}

written_explanations! {
    "L0101",
    "L0102",
    "L0103",
    "L0104",
    "L0105",
    "L0106",
    "L0107",
    "L0201",
    "L0202",
    "L0203",
    "L0204",
    "L0205",
    "L0206",
    "L0207",
    "L0208",
    "L0209",
    "L0210",
    "L0211",
    "L0212",
    "L0213",
    "L0214",
    "L0215",
    "L0216",
    "L0217",
    "L0218",
    "L0219",
    "L0220",
    "L0221",
    "L0222",
    "L0223",
    "L0224",
    "L0225",
    "L0226",
    "L0227",
    "L0228",
    "L0229",
    "L0230",
    "L0231",
    "L0233",
    "L0234",
    "L0235",
    "L0236",
    "L0237",
    "L0238",
    "L0239",
    "L0240",
    "L0241",
    "L0242",
    "L0243",
    "L0244",
    "L0245",
    "L0246",
    "L0247",
    "L0248",
    "L0249",
    "L0250",
    "L0251",
    "L0252",
    "L0253",
    "L0255",
    "L0256",
    "L0257",
    "L0301",
    "L0302",
    "L0303",
    "L0304",
    "L0305",
    "L0306",
    "L0307",
    "L0308",
    "L0309",
    "L0311",
    "L0312",
    "L0313",
    "L0314",
    "L0315",
    "L0316",
    "L0317",
    "L0318",
    "L0319",
    "L0320",
    "L0321",
    "L0322",
    "L0323",
    "L0324",
    "L0325",
    "L0326",
    "L0327",
    "L0328",
    "L0329",
    "L0330",
    "L0331",
    "L0332",
    "L0333",
    "L0334",
    "L0335",
    "L0336",
    "L0338",
    "L0339",
    "L0340",
    "L0341",
    "L0342",
    "L0343",
    "L0344",
    "L0345",
    "L0346",
    "L0347",
    "L0348",
    "L0349",
    "L0350",
    "L0351",
    "L0352",
    "L0353",
    "L0354",
    "L0355",
    "L0356",
    "L0357",
    "L0358",
    "L0359",
    "L0360",
    "L0361",
    "L0362",
    "L0363",
    "L0401",
    "L0402",
    "L0403",
    "L0404",
    "L0406",
    "L0407",
    "L0408",
    "L0409",
    "L0410",
    "L0411",
    "L0412",
    "L0413",
    "L0414",
    "L0415",
    "L0416",
    "L0417",
    "L0418",
    "L0419",
    "L0421",
    "L0422",
    "L0423",
    "L0424",
    "L0425",
    "L0426",
    "L0428",
    "L0429",
    "L0430",
    "L0431",
    "L0432",
    "L0433",
    "L0435",
    "L0436",
    "L0437",
    "L0438",
    "L0439",
    "L0440",
    "L0441",
    "L0442",
    "L0443",
    "L0444",
    "L0445",
    "L0446",
    "L0447",
    "L0448",
    "L0449",
    "L0450",
    "L0451",
    "L0452",
    "L0453",
    "L0454",
    "L0455",
    "L0456",
    "L0457",
    "L0458",
    "L0459",
    "L0460",
    "L0461",
    "L0462",
    "L0463",
    "L0464",
    "L0465",
    "L0501",
    "L0503",
    "L0505",
    "L0506",
    "L0507",
    "L0509",
    "L0510",
    "L0511",
    "L0512",
    "L0514",
    "L0515",
    "L0516",
    "L0517",
    "L0601",
    "L0602",
    "L0603",
    "L0604",
    "L0605",
    "L0606",
}

pub struct Explanation {
    code: DiagnosticCode,
    constant: &'static str,
}

impl Explanation {
    const CONSTANT_WORD_SEPARATOR: char = '_';
    const DISPLAY_WORD_SEPARATOR: &str = " ";

    pub fn of_code(code: &str) -> Option<Self> {
        Self::all().find(|explanation| explanation.code.as_str() == code)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        DiagnosticCode::named().map(|(constant, code)| Self { code, constant })
    }

    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    pub fn display_name(&self) -> String {
        self.constant
            .replace(Self::CONSTANT_WORD_SEPARATOR, Self::DISPLAY_WORD_SEPARATOR)
            .to_lowercase()
    }

    pub fn text(&self) -> Option<&'static str> {
        Self::WRITTEN
            .iter()
            .find(|(code, _)| *code == self.code.as_str())
            .map(|(_, text)| *text)
    }
}

#[cfg(test)]
mod tests {
    use super::Explanation;

    #[test]
    fn the_display_name_spells_the_constant_in_lowercase_words() {
        let explanation = Explanation::of_code("L0501").expect("L0501 is a code");
        assert_eq!(explanation.display_name(), "use after move");
    }

    #[test]
    fn every_written_explanation_names_a_code_of_the_table() {
        for (code, _) in Explanation::WRITTEN {
            assert!(Explanation::of_code(code).is_some(), "{code} is not a code");
        }
    }

    #[test]
    fn a_code_outside_the_table_has_no_explanation() {
        assert!(Explanation::of_code("L0999").is_none());
    }
}
