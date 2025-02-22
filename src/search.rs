use std::fmt::Debug;

use crate::authentication::Authentication;
use grep::regex::RegexMatcherBuilder;
use log::info;

use self::github::GithubSearcher;

use {
    grep::{
        cli,
        printer::{ColorSpecs, StandardBuilder},
        searcher::{BinaryDetection, SearcherBuilder},
    },
    log::error,
    std::{error::Error, ffi::OsString, io::IsTerminal},
    termcolor::ColorChoice,
    walkdir::WalkDir,
};

pub mod github;

#[derive(Debug)]
pub struct Searcher<T: Authentication> {
    pub github: GithubSearcher<T>,
}

impl<T: Authentication> Searcher<T> {
    pub fn new(github: GithubSearcher<T>) -> Self {
        return Self { github };
    }
    pub async fn initialise(&self) {
        self.github.initialise_octocrab().await;
    }
}

//Original inspiration: https://github.com/BurntSushi/ripgrep/blob/master/crates/grep/examples/simplegrep.rs
pub fn search(
    pattern: &str,
    paths: &[OsString],
    case_insensitve: bool,
) -> Result<(), Box<dyn Error>> {
    let matcher = RegexMatcherBuilder::new()
        .line_terminator(Some(b'\n'))
        .case_insensitive(case_insensitve)
        .build(pattern)?;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build();
    let mut printer = StandardBuilder::new()
        .color_specs(ColorSpecs::default_with_color())
        .heading(true)
        .build(cli::stdout(if std::io::stdout().is_terminal() {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        }));

    for path in paths {
        for result in WalkDir::new(path) {
            let dir_entry = match result {
                Ok(entry) => entry,
                Err(e) => {
                    error!("Error walking directory: {}\n{e}", path.to_str().unwrap());
                    continue;
                }
            };
            if !dir_entry.file_type().is_file() {
                continue;
            }
            let result = searcher.search_path(
                &matcher,
                dir_entry.path(),
                printer.sink_with_path(&matcher, dir_entry.path()),
            );
            if let Err(e) = result {
                error!("{}: {e}", dir_entry.path().display());
            }
        }
    }
    return Ok(());
}
