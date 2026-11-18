use clap::Args;

use super::git_setup::GitSetup;
use super::github_workflow::GithubWorkflow;
use super::project_component::ProjectComponent;

#[derive(Args)]
pub struct NewComponents {
    #[arg(long, help = "Write no `.gitignore` and run no `git init`")]
    skip_git: bool,
    #[arg(long, help = "Write no GitHub Actions workflow")]
    skip_github: bool,
}

impl NewComponents {
    pub fn chosen(&self) -> Vec<Box<dyn ProjectComponent>> {
        let offered: [(bool, Box<dyn ProjectComponent>); 2] = [
            (self.skip_git, Box::new(GitSetup)),
            (self.skip_github, Box::new(GithubWorkflow)),
        ];
        offered
            .into_iter()
            .filter(|(skipped, _)| !skipped)
            .map(|(_, component)| component)
            .collect()
    }
}
