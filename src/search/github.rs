use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use git_cloner::github::{GitClone, GitCloner};
use git_cloner::github_authentication::authentication::Authentication;
use octocrab::models::Repository;
use tokio::task::JoinSet;

#[derive(Debug)]
pub struct GithubSearcher<T: Authentication> {
    pub cloner: GitCloner<T>,
    pub branches: Vec<String>,
    pub owner: String,
}

impl<T: Authentication> GithubSearcher<T> {
    pub fn new(
        authentication: T,
        repositories_directory_path: PathBuf,
        branches: Vec<String>,
        owner: String,
    ) -> Result<Self> {
        let _ = fs::create_dir_all(&repositories_directory_path);
        let cloner = GitCloner::new(authentication, repositories_directory_path)?;
        Ok(Self {
            cloner,
            branches,
            owner,
        })
    }

    pub async fn update_repositories(&self, repo_prefix: &str) -> Result<Vec<Result<()>>> {
        let owner = self.owner.clone();
        let repositories = Self::get_repos(&owner)
            .await
            .with_context(|| "Failed to clone and fetch all repositories")?
            .into_iter()
            .filter(|repo| repo.name.starts_with(repo_prefix))
            .collect::<Vec<Repository>>();

        let git_clones: Vec<GitClone> = if self.branches.is_empty() {
            repositories
                .into_iter()
                .map(|repo| GitClone::new(owner.clone(), repo.name, None))
                .collect::<Vec<GitClone>>()
        } else {
            let mut tasks: JoinSet<Result<Vec<GitClone>>> = JoinSet::new();
            repositories.into_iter().for_each(|repo| {
                let name = repo.name;
                let owner = owner.clone();
                let branches = self.branches.clone();
                tasks.spawn(async move {
                    let octocrab_instance = octocrab::instance();
                    let branch_pages = octocrab_instance
                        .repos(owner.clone(), name.clone())
                        .list_branches()
                        .send()
                        .await?;
                    let clones = octocrab_instance
                        .all_pages(branch_pages)
                        .await?
                        .into_iter()
                        .filter(|branch| branches.contains(&branch.name))
                        .map(|branch| GitClone::new(owner.clone(), name.clone(), Some(branch.name)))
                        .collect();
                    Ok(clones)
                });
            });
            tasks
                .join_all()
                .await
                .into_iter()
                .filter_map(|git_clone| git_clone.ok())
                .flatten()
                .collect()
        };
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
