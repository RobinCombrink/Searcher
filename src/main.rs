use std::{fs, path::PathBuf};

use crate::search::{github::GithubSearcher, Searcher};
use anyhow::Result;
use git_cloner::github_authentication::authentication::GitHubCliAuthentication;
use std::io::Write;

use {clap::Parser, env_logger, std::ffi::OsString};

mod search;

const DEFAULT_FILES_TO_SEARCH_DIRECTORY: &str = "FilesToSearch";
const DEFAULT_GITHUB_REPOSITORIES_DIRECTORY: &str = "github";

#[derive(Parser, Debug)]
struct SearchArguments {
    #[clap(short = 'p', long, default_value = "flutter_")]
    repository_filter_prefix: String,
    #[clap(short = 'o', long, default_value = "flutter")]
    github_organisation: String,
    #[clap(short = 's', long, default_value = "main")]
    search_pattern: OsString,
    #[clap(short = 'i', long, num_args = 0, default_value_t = true)]
    ignore_case: bool,
    #[clap(short = 'u', long, default_value = "RobinCombrink")]
    github_username: String,
    #[clap(short = 'b', long, value_delimiter = ',', default_value = None)]
    branches: Vec<String>,
    #[clap(short = 'l', long, default_value = DEFAULT_FILES_TO_SEARCH_DIRECTORY)]
    local_search_directory: PathBuf,
}

#[derive(Parser, Debug)]
struct ClearArguments {
    #[clap(short = 'l', long, default_value = DEFAULT_FILES_TO_SEARCH_DIRECTORY)]
    local_search_directory: PathBuf,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Quickly search through a number of repositories hosted on github"
)]
enum CliCommand {
    #[clap(about = "Clone a set of github repositories and then search through the context")]
    Search(SearchArguments),
    #[clap(about = "Delete all files and directories recursively from the provided directory")]
    Clear(ClearArguments),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: CliCommand = CliCommand::parse();

    setup_logging();

    match args {
        CliCommand::Search(search_arguments) => search(search_arguments).await,
        CliCommand::Clear(clear_arguments) => clear(clear_arguments),
    }
}

fn clear(args: ClearArguments) -> Result<()> {
    Ok(fs::remove_dir_all(args.local_search_directory)?)
}

async fn search(args: SearchArguments) -> Result<()> {
    let authentication = GitHubCliAuthentication::new(args.github_username)?;

    let local_search_directory_github = args
        .local_search_directory
        .join(DEFAULT_GITHUB_REPOSITORIES_DIRECTORY);

    let github_updater = GithubSearcher::new(
        authentication,
        local_search_directory_github.clone(),
        args.branches,
        args.github_organisation,
    )?;

    let _ = fs::create_dir_all(local_search_directory_github.clone());

    github_updater
        .update_repositories(&args.repository_filter_prefix)
        .await?
        .into_iter()
        .collect::<Result<Vec<()>>>()?;

    let searcher = Searcher::new(args.ignore_case, &args.search_pattern)?;

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
