use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use git_cloner::github::{GitClone, GitCloner};
use git_cloner::github_authentication::authentication::Authentication;
use octocrab::models::Repository;

#[derive(Debug)]
pub struct GithubSearcher<T: Authentication> {
    pub cloner: GitCloner<T>,
    pub owner: String,
}

impl<T: Authentication> GithubSearcher<T> {
    pub fn new(
        authentication: T,
        repositories_directory_path: PathBuf,
        directory: PathBuf,
        owner: String,
    ) -> Self {
        let _ = fs::create_dir_all(repositories_directory_path.join(&directory));
        let cloner = GitCloner::<T> {
            authentication,
            directory_path: repositories_directory_path,
        };
        Self { cloner, owner }
    }

    pub async fn update_repositories(&self, repo_prefix: &str) -> Result<Vec<Result<()>>> {
        let repositories = Self::get_repos(&self.owner)
            .await
            .with_context(|| "Failed to clone and fetch all repositories")?;

        let git_clones = repositories
            .iter()
            .filter(|repo| repo.name.starts_with(repo_prefix))
            .collect::<Vec<&Repository>>()
            .into_iter()
            .map(|repo| GitClone::new(self.owner.clone(), repo.name.clone()))
            .collect();

        Ok(self.cloner.clone_or_fetch_repositories(git_clones).await)
    }

    async fn get_repos(owner: &str) -> Result<Vec<Repository>> {
        let octocrab_instance = octocrab::instance();
        let repo_page = octocrab_instance
            .orgs(owner)
            .list_repos()
            .send()
            .await
            .with_context(|| format!("Failed to list repositories for organisation: {owner}"))?;
        let results = octocrab_instance.all_pages(repo_page).await?;
        return Ok(results);
    }
}
