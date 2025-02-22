use anyhow::{Context, Result};
use log::error;
use secrecy::SecretString;
use std::{
    process::{exit, Command, Output},
    sync::LazyLock,
};

const DEFAULT_SHELL: &str = "cmd";

pub trait Authentication {
    fn get_token(&self) -> SecretString;
    fn get_username(&self) -> String;
}

pub struct GitHubCliAuthentication {
    pub(crate) token: LazyLock<SecretString>,
    pub(crate) username: String,
}

impl GitHubCliAuthentication {
    pub fn new(username: String) -> Self {
        Self {
            token: LazyLock::new(|| Self::get_token(DEFAULT_SHELL)),
            username,
        }
    }
    fn get_token(shell: &str) -> SecretString {
        let args = vec![
            "/C".into(),
            "gh".to_string(),
            "auth".to_string(),
            "token".to_string(),
        ];
        let result: Result<Output> = Command::new(shell).args(&args).output().with_context(|| {
            format!(
                "Something went wrong executing the command: {:#?} in the program {}",
                args, shell
            )
        });

        let token = match result {
            Ok(token) => String::from_utf8(token.stdout)
                .expect("Not utf8 token")
                .trim()
                .to_owned()
                .into(),
            Err(err) => {
                error!("Could not get token:\n, {:#?}", err);
                exit(1)
            }
        };
        token
    }
}

impl Authentication for GitHubCliAuthentication {
    fn get_token(&self) -> SecretString {
        (*self.token).clone()
    }

    fn get_username(&self) -> String {
        self.username.clone()
    }
}
