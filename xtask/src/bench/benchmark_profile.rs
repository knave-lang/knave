pub struct BenchmarkProfile;

impl BenchmarkProfile {
    const NAME: &'static str = "benchmark";
    const SETTINGS: [(&'static str, &'static str); 3] = [
        ("inherits", "\"release\""),
        ("codegen-units", "1"),
        ("lto", "false"),
    ];
    const PROFILE_FLAG: &'static str = "--profile";
    const CONFIG_FLAG: &'static str = "--config";

    pub fn cargo_arguments(&self) -> impl Iterator<Item = String> {
        self.selection().chain(self.definition())
    }

    fn selection(&self) -> impl Iterator<Item = String> {
        [Self::PROFILE_FLAG, Self::NAME]
            .into_iter()
            .map(String::from)
    }

    fn definition(&self) -> impl Iterator<Item = String> {
        Self::SETTINGS.into_iter().flat_map(|(key, value)| {
            [
                Self::CONFIG_FLAG.to_string(),
                format!("profile.{}.{key}={value}", Self::NAME),
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ProfileFixture;

    impl ProfileFixture {
        fn configured_settings() -> Vec<String> {
            let arguments: Vec<_> = BenchmarkProfile.cargo_arguments().collect();
            arguments
                .windows(2)
                .filter(|pair| pair[0] == "--config")
                .map(|pair| pair[1].clone())
                .collect()
        }
    }

    #[test]
    fn selects_the_benchmark_profile() {
        let arguments: Vec<_> = BenchmarkProfile.cargo_arguments().collect();
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--profile", "benchmark"])
        );
    }

    #[test]
    fn defines_release_with_one_codegen_unit_and_no_link_time_optimization() {
        assert_eq!(
            ProfileFixture::configured_settings(),
            [
                "profile.benchmark.inherits=\"release\"",
                "profile.benchmark.codegen-units=1",
                "profile.benchmark.lto=false",
            ]
        );
    }
}
