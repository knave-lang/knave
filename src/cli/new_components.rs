use clap::Args;

use super::git_setup::GitSetup;
use super::project_component::ProjectComponent;

#[derive(Args)]
pub struct NewComponents {
    #[arg(long, help = "Write no `.gitignore` and run no `git init`")]
    skip_git: bool,
}

impl NewComponents {
    pub fn chosen(&self) -> Vec<Box<dyn ProjectComponent>> {
        let offered: [(bool, Box<dyn ProjectComponent>); 1] = [(self.skip_git, Box::new(GitSetup))];
        offered
            .into_iter()
            .filter(|(skipped, _)| !skipped)
            .map(|(_, component)| component)
            .collect()
    }
}
