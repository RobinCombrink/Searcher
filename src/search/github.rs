use std::collections::HashSet;
use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use futures::{stream, StreamExt};
use git2::build::RepoBuilder;
use git2::{BranchType, Cred, FetchOptions, RemoteCallbacks};
use indicatif::ProgressBar;
use log::{debug, error, info};
use octocrab::models::Repository;
use octocrab::Octocrab;
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use url::Url;

use crate::authentication::Authentication;
use crate::logging;

const REMOTE_NAME: &str = "origin";

#[derive(Debug)]
pub struct GithubSearcher<T: Authentication> {
    pub directory: PathBuf,
    pub owner: String,
    pub base_path: PathBuf,
    pub authentication: T,
}

impl<T: Authentication> GithubSearcher<T> {
    pub fn new(authentication: T, base_path: PathBuf, directory: PathBuf, owner: String) -> Self {
        let _ = fs::create_dir_all(base_path.join(&directory));
        let github = Self {
            directory,
            owner,
            base_path,
            authentication,
        };

        github.initialise_octocrab();

        github
    }

    pub fn initialise_octocrab(&self) {
        let token = self.authentication.get_token().expose_secret().to_owned();
        match Octocrab::builder().personal_token(token).build() {
            Ok(instance) => octocrab::initialise(instance),
            Err(e) => logging::print_error_and_exit(&format!(
                "Failed to build Octocrab instance - probably a bad token: {e}"
            )),
        };
    }

    pub async fn update_repositories(
        &self,
        github_directory: PathBuf,
        repo_prefix: &str,
    ) -> Result<()> {
        let repositories = Self::get_repos(&self.owner)
            .await
            .with_context(|| "Failed to clone and fetch all repositories")?;

        let filtered_repositories = repositories
            .iter()
            .filter(|repo| repo.name.starts_with(repo_prefix))
            .collect::<Vec<&Repository>>();

        self.clone_or_fetch_repositories(&github_directory, &filtered_repositories, &self.base_path)
            .await
    }

    async fn clone_or_fetch_repositories(
        &self,
        repositories_directory: &Path,
        repositories: &[&Repository],
        path: &Path,
    ) -> Result<()> {
        info!("Updating {} repo(s)", &repositories.len());

        let semaphore = Arc::new(Semaphore::new(10));

        let token = self.authentication.get_token();

        let futures = repositories.into_iter().map(|repo| {
            let local_path = path.join(&repositories_directory).join(&repo.name);
            let semaphore = Arc::clone(&semaphore);
            let token = token.clone();
            async move {
                let _permit = semaphore.acquire_owned().await.unwrap();
                match git2::Repository::open(&local_path) {
                    Ok(local_repo) => {
                        self.fetch_repository(
                            local_repo,
                            repo.name.to_owned(),
                            local_path.to_owned(),
                            token,
                        )
                        .await
                    }
                    Err(_) => {
                        self.clone_repository(
                            repo.name.to_owned(),
                            repo.html_url.to_owned(),
                            local_path.to_owned(),
                            token,
                        )
                        .await
                    }
                }
            }
        });

        //TODO: Make the 10 a settable argument
        let results: Vec<_> = stream::iter(futures).buffer_unordered(5).collect().await;
        let mut success: usize = 0;
        let mut failure: usize = 0;
        for result in results {
            match result? {
                Ok(()) => {
                    success += 1;
                }
                Err(_) => {
                    failure += 1;
                    //TODO: Error
                }
            }
        }
        info!("Success: {success}\nFailure: {failure}");
        if failure > 0 {
            info!("Failed to clone or fetch all repositories. Please retry");
            exit(1);
        }
        Ok(())
    }

    fn fetch_repository(
        &self,
        repo: git2::Repository,
        name: String,
        path: PathBuf,
        token: SecretString,
    ) -> tokio::task::JoinHandle<Result<()>> {
        info!("Fetching {name}, in {}", path.display());
        let branches: HashSet<&str> = ["origin/main", "origin/master", "origin/develop"]
            .iter()
            .cloned()
            .collect();

        let remote_branches = repo.branches(Some(BranchType::Remote)).unwrap();
        let filtered_branches = remote_branches
            .filter_map(|branch| {
                //TODO: Make safe
                let branch = branch.as_ref().unwrap();
                let branch_name = &branch.0.name().unwrap().unwrap();
                //TODO: Add verbose logging option
                if branches.contains(branch_name) {
                    // println!("branches contains: {branch_name}");
                    return Some((*branch_name).to_owned());
                } else {
                    // println!("branches: {:?} does not contain: {branch_name}", branches);
                    return None;
                }
            })
            .collect::<Vec<String>>();

        let username = self.authentication.get_username().clone();
        tokio::task::spawn_blocking(move || {
            Self::fetch_repo_sync(filtered_branches, path, repo, username, token)
        })
    }

    fn fetch_repo_sync(
        filtered_branches: Vec<String>,
        path: PathBuf,
        repo: git2::Repository,
        username: String,
        token: SecretString,
    ) -> Result<()> {
        let mut remote = repo.find_remote(REMOTE_NAME)?;
        // error!("Failed to find remote for:{}/\n{e}", path.display());
        for branch in &filtered_branches {
            if let Some(p) = path.to_str() {
                if path.exists() {
                    let mut fetch_options =
                        Self::create_repository_fetch_options(&token, p.to_owned(), &username);

                    match remote.fetch(&[branch], Some(&mut fetch_options), None) {
                        Ok(_) => info!("Successfully fetched: {p}/{branch}",),
                        Err(e) => {
                            error!("Failed to fetch: {p}/{branch}:\n {e}",)
                        }
                    }
                } else {
                    error!("Path {p} does not exist",);
                    return Err(anyhow!("Path {p} does not exist"));
                }
            } else {
                error!("File path to_str was NONE");
            }
        }
        return Ok(());
    }

    fn create_repository_fetch_options<'a>(
        token: &'a SecretString,
        name: String,
        username: &'a str,
    ) -> FetchOptions<'a> {
        let pb = ProgressBar::new(100);

        let mut callbacks = RemoteCallbacks::new();
        let mut last_logged_progress = None;
        callbacks.transfer_progress(move |progress| {
            let progress_percent = ((progress.received_objects() as f32
                / progress.total_objects() as f32)
                * 100 as f32)
                .ceil() as u64;
            if progress_percent % 5 == 0 && Some(progress_percent) != last_logged_progress {
                pb.inc(2);
                info!(
                    "\nRepo:\t\t{name}\nProgress:\t{}/{} objects\n\tProgress: {progress_percent}",
                    progress.received_objects(),
                    progress.total_objects()
                );
                last_logged_progress = Some(progress_percent);
            }
            true
        });

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::userpass_plaintext(
                username_from_url.unwrap_or_else(|| username),
                token.expose_secret(),
            )
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        fetch_options.depth(1);

        return fetch_options;
    }

    fn clone_repository(
        &self,
        name: String,
        url: Option<Url>,
        path: PathBuf,
        token: SecretString,
    ) -> JoinHandle<Result<()>> {
        let username = self.authentication.get_username().clone();
        tokio::task::spawn_blocking(move || Self::clone_repo_sync(name, url, path, token, username))
    }

    fn clone_repo_sync(
        name: String,
        url: Option<Url>,
        path: PathBuf,
        token: SecretString,
        username: String,
    ) -> Result<()> {
        info!("Cloning {name} into {}", &path.display());

        let _ = fs::create_dir_all(&path);
        let fetch_options = Self::create_repository_fetch_options(&token, name, &username);
        match path.to_str() {
            Some(path_str) => match url {
                Some(url) => match RepoBuilder::new()
                    .fetch_options(fetch_options)
                    .clone(url.as_str(), &path)
                {
                    Ok(_) => {
                        info!("Successfully cloned {url} into {path_str}");
                        Ok(())
                    }
                    Err(e) => {
                        let _ = fs::remove_dir_all(&path);
                        info!("Failed to clone repo:\n{url}\n into {path_str}: {e}");
                        Err(e.into())
                    }
                },
                //TODO: Maybe don't exit
                None => logging::print_error_and_exit("HTML URL was empty: exiting"),
            },
            None => {
                debug!("File path was NONE");
                Err(anyhow!("File path was none".to_string()))
            }
        }
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
