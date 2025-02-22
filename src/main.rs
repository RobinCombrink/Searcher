use std::{fmt::Display, fs};

use crate::search::{github::GithubSearcher, Searcher};
use authentication::GitHubCliAuthentication;
use clap::ValueEnum;
use grep::cli;
use log::{error, info};
use std::io::Write;

use {
    clap::Parser,
    env_logger,
    log::trace,
    octocrab::{self, Error as OctoError},
    std::{ffi::OsString, path::Path},
};
pub mod authentication;
pub mod logging;
pub mod search;

const DEFAULT_FILES_TO_SEARCH_DIRECTORY: &str = "FilesToSearch";
const DEFAULT_GITHUB_REPOSITORIES_DIRECTORY: &str = "github";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CLIArguments {
    #[clap(short = 'p', long = "prefix", default_value = "")]
    prefix: String,
    #[clap(short = 'o', long = "owner", default_value = "RobinCombrink")]
    owner: String,
    #[clap(short = 's', long = "search-term", default_value = "main")]
    search_term: OsString,
    #[clap(short = 'i', long = "case-insensitve", num_args = 0)]
    ignore_case: bool,
    #[clap(short = 'l', long = "log-level", value_enum, default_value_t=logging::LevelFilter::Info)]
    min_log_level: logging::LevelFilter,
}

#[tokio::main]
async fn main() {
    use std::time::Instant;
    let now = Instant::now();
    {
        //TODO: Extract to CLI argument
        let path = Path::new(DEFAULT_FILES_TO_SEARCH_DIRECTORY);
        //TODO: Extract to CLI argument
        let github_directory = DEFAULT_GITHUB_REPOSITORIES_DIRECTORY;
        //TODO: Extract to CLI argument
        let args: CLIArguments = CLIArguments::parse();

        setup_logging(args.min_log_level);
        trace!("Logging setup successful");

        let authentication = GitHubCliAuthentication::default();

        let searcher = Searcher::new(GithubSearcher::new(
            authentication,
            path.to_path_buf(),
            github_directory.into(),
            args.owner,
        ));

        let _ = fs::create_dir_all(path.join(github_directory));

        searcher
            .github
            .update_repositories(github_directory.into(), &args.prefix)
            .await;

        // TODO Extract to Search module
        match cli::pattern_from_os(&args.search_term) {
            Ok(p) => match search::search(p, &[path.as_os_str().to_owned()], args.ignore_case) {
                Ok(()) => info!("Search successful"),
                Err(e) => error!("Search unsuccessful: {e}"),
            },
            Err(e) => logging::print_error_and_exit(&format!(
                "The provided search pattern was not valid: {e}"
            )),
        };
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}

fn setup_logging(level_filter: logging::LevelFilter) {
    env_logger::builder()
        .filter_level(level_filter.into())
        .parse_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                //TODO: Date and time?
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
}
