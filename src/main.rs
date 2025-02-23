use std::fs;

use crate::search::{github::GithubSearcher, Searcher};
use anyhow::Result;
use git_cloner::github_authentication::authentication::GitHubCliAuthentication;
use std::io::Write;

use {
    clap::Parser,
    env_logger,
    log::trace,
    std::{ffi::OsString, path::Path},
};

mod search;

const DEFAULT_FILES_TO_SEARCH_DIRECTORY: &str = "FilesToSearch";
const DEFAULT_GITHUB_REPOSITORIES_DIRECTORY: &str = "github";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CLIArguments {
    #[clap(short = 'p', long = "prefix", default_value = "flutt")]
    prefix: String,
    #[clap(short = 'o', long = "owner", default_value = "flutter")]
    owner: String,
    #[clap(short = 's', long = "search-term", default_value = "main")]
    search_term: OsString,
    #[clap(short = 'i', long = "case-insensitve", num_args = 0)]
    ignore_case: bool,
    #[clap(short = 'u', long = "github-username", default_value = "RobinCombrink")]
    github_username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    use std::time::Instant;
    let now = Instant::now();
    {
        //TODO: Extract to CLI argument
        let path = Path::new(DEFAULT_FILES_TO_SEARCH_DIRECTORY);
        //TODO: Extract to CLI argument
        let github_directory = DEFAULT_GITHUB_REPOSITORIES_DIRECTORY;
        //TODO: Extract to CLI argument
        let args: CLIArguments = CLIArguments::parse();

        setup_logging();
        trace!("Logging setup successful");

        let authentication = GitHubCliAuthentication::new(args.github_username)?;

        let github_searcher = GithubSearcher::new(
                authentication,
            path.to_path_buf().join(github_directory),
                args.owner,
        )?;

        let searcher = Searcher::new(github_searcher, args.ignore_case, &args.search_term)?;

        let _ = fs::create_dir_all(path.join(github_directory));

        searcher
            .github
            .update_repositories(&args.prefix)
            .await?
            .into_iter()
            .collect::<Result<Vec<()>>>()?;

        // searcher.search(&[path.as_os_str().to_owned()])?;
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    Ok(())
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
