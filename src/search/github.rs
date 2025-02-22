use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use git_cloner::github::{GitCloner, GitRepository};
use git_cloner::github_authentication::authentication::Authentication;
use log::info;
use octocrab::models::Repository;

struct GitClone {
    owner: String,
    repo: String,
}

impl GitClone {
    fn new(owner: String, repo: String) -> Self {
        Self { owner, repo }
    }
}

impl GitRepository for GitClone {
    fn get_owner(&self) -> String {
        self.owner.clone()
    }
    fn get_repository_name(&self) -> String {
        self.repo.clone()
    }
}

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

    pub async fn update_repositories(
        &self,
        repo_prefix: &str,
    ) -> Result<()> {
        let repositories = Self::get_repos(&self.owner)
            .await
            .with_context(|| "Failed to clone and fetch all repositories")?;

        let filtered_repositories = repositories
            .iter()
            .filter(|repo| repo.name.starts_with(repo_prefix))
            .collect::<Vec<&Repository>>();

        self.clone_or_fetch_repositories(&filtered_repositories)
            .await
    }

    async fn clone_or_fetch_repositories(
        &self,
        repositories: &[&Repository],
    ) -> Result<()> {
        info!("Updating {} repo(s)", &repositories.len());

        let git_clones = repositories
            .into_iter()
            .map(|repo| GitClone::new(self.owner.clone(), repo.name.clone()))
            .collect();

        self.cloner.clone_or_fetch_repositories(git_clones).await;

        Ok(())
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
