use std::{fs, path::PathBuf};

use crate::search::{github::GithubSearcher, Searcher};
use anyhow::Result;
use git_cloner::github_authentication::authentication::GitHubCliAuthentication;
use std::io::Write;

use {clap::Parser, env_logger, log::trace, std::ffi::OsString};

mod search;

const DEFAULT_FILES_TO_SEARCH_DIRECTORY: &str = "FilesToSearch";
const DEFAULT_GITHUB_REPOSITORIES_DIRECTORY: &str = "github";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CLIArguments {
    #[clap(short = 'p', long = "prefix", default_value = "flutter_")]
    prefix: String,
    #[clap(short = 'o', long = "owner", default_value = "flutter")]
    owner: String,
    #[clap(short = 's', long = "search-term", default_value = "main")]
    search_term: OsString,
    #[clap(short = 'i', long = "case-insensitve", num_args = 0)]
    ignore_case: bool,
    #[clap(short = 'u', long = "github-username", default_value = "RobinCombrink")]
    github_username: String,
    #[clap(short = 'l', long = "local-search-directory", default_value = DEFAULT_FILES_TO_SEARCH_DIRECTORY)]
    local_search_directory: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: CLIArguments = CLIArguments::parse();

    setup_logging();

    let authentication = GitHubCliAuthentication::new(args.github_username)?;

    let local_search_directory_github = args
        .local_search_directory
        .join(DEFAULT_GITHUB_REPOSITORIES_DIRECTORY);

    let github_updater = GithubSearcher::new(
        authentication,
        local_search_directory_github.clone(),
        args.owner,
    )?;

    let _ = fs::create_dir_all(local_search_directory_github.clone());

    github_updater
        .update_repositories(&args.prefix)
        .await?
        .into_iter()
        .collect::<Result<Vec<()>>>()?;

    let searcher = Searcher::new(args.ignore_case, &args.search_term)?;

    searcher.search(&[local_search_directory_github.as_os_str().to_owned()])
}

fn setup_logging() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
}
