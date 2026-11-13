//! The repositories the token's account can see: its own, those it collaborates on and its
//! organisations', newest pushed first.

use adapter_traits::{AccountRepositories, ForgeRepository};
use serde::Deserialize;

use crate::github_api::{ApiFailure, GitHubApi};

/// The most a page holds.
const PER_PAGE: usize = 100;

#[derive(Deserialize)]
struct Listed {
    name: String,
    owner: Owner,
    clone_url: String,
    private: bool,
    pushed_at: Option<String>,
}

#[derive(Deserialize)]
struct Owner {
    login: String,
}

pub(super) fn repositories(api: &GitHubApi, most: usize) -> Result<AccountRepositories, ApiFailure> {
    let path = format!("/user/repos?sort=pushed&per_page={PER_PAGE}");
    let paged = api.get_paged(&path, most.div_ceil(PER_PAGE).max(1))?;
    let mut repositories = Vec::new();
    for page in &paged.pages {
        let listed: Vec<Listed> = page.decode("the repositories of an account")?;
        repositories.extend(listed.into_iter().map(|one| ForgeRepository {
            owner: one.owner.login,
            name: one.name,
            url: one.clone_url,
            private: one.private,
            pushed_at: one.pushed_at,
        }));
    }
    let cut = paged.cut || repositories.len() > most;
    repositories.truncate(most);
    Ok(AccountRepositories { repositories, cut })
}
