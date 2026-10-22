//! A GraphQL list read to its end: the page the query came back with, then the pages after its
//! cursor, up to a cap, saying where the cap cut.
//!
//! `gh` reads these lists to their ends, so a reader that stopped at the first hundred would
//! answer from less than `gh` does without saying so.

use serde::{Deserialize, Serialize};

use super::page::Asked;
use crate::github_api::{ApiFailure, GitHubRepository};

/// The most pages of one list read: 1,000 of a thing.
pub(super) const MOST_PAGES: usize = 10;

/// A list with its length on the forge, and where the next page starts.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Connection<T> {
    pub(super) total_count: Option<usize>,
    page_info: Option<PageInfo>,
    nodes: Option<Vec<Option<T>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: Option<bool>,
    end_cursor: Option<String>,
}

/// `{ more: <list> }`, the alias a query gives a later page.
#[derive(Deserialize)]
pub(super) struct More<T> {
    pub(super) more: Option<Connection<T>>,
}

/// What a query for a later page of a pull request's list is given.
#[derive(Serialize)]
pub(super) struct AskedAfter<'a> {
    pub(super) owner: &'a str,
    pub(super) name: &'a str,
    pub(super) number: u64,
    pub(super) after: &'a str,
}

/// A query for a later page of something on a pull request, `inside` being what to read of it.
pub(super) fn later_page(inside: &str) -> String {
    format!(
        "query($owner:String!,$name:String!,$number:Int!,$after:String!){{\
         repository(owner:$owner,name:$name){{pullRequest(number:$number){{{inside}}}}}}}"
    )
}

/// What a list came to.
pub(super) struct Listed<T> {
    pub(super) rows: Vec<T>,
    /// The list had a page the cap stopped short of.
    pub(super) cut: bool,
    /// Its length on the forge, where the first page said.
    pub(super) total: Option<usize>,
}

impl<T> Connection<T> {
    /// The rows of this page, and the cursor of the next where there is one.
    fn page(self) -> (Vec<T>, Option<String>) {
        let next = self.page_info.and_then(|it| if it.has_next_page == Some(true) { it.end_cursor } else { None });
        (self.nodes.unwrap_or_default().into_iter().flatten().collect(), next)
    }
}

/// `first`, then `more(cursor)` for each page after it, `most_pages` in all. A page that comes
/// back as nothing ends the list.
pub(super) fn read_to_the_end<T>(
    first: Option<Connection<T>>,
    most_pages: usize,
    mut more: impl FnMut(&str) -> Result<Option<Connection<T>>, ApiFailure>,
) -> Result<Listed<T>, ApiFailure> {
    let Some(first) = first else { return Ok(Listed { rows: Vec::new(), cut: false, total: None }) };
    let total = first.total_count;
    let (mut rows, mut next) = first.page();
    let mut pages = 1;
    let mut cut = false;
    while let Some(cursor) = next.take() {
        if pages >= most_pages {
            cut = true;
            break;
        }
        let Some(page) = more(&cursor)? else { break };
        let (more_rows, after) = page.page();
        rows.extend(more_rows);
        next = after;
        pages += 1;
    }
    Ok(Listed { rows, cut, total })
}

/// A repository's owner and name apart, which every query about one of its pull requests names.
pub(super) struct Named {
    owner: String,
    name: String,
}

impl Named {
    pub(super) fn of(repository: &GitHubRepository) -> Named {
        let slug = repository.address().slug();
        let (owner, name) = slug.split_once('/').unwrap_or((&slug, ""));
        Named { owner: owner.to_string(), name: name.to_string() }
    }

    pub(super) fn asked(&self, number: u64) -> Asked<'_> {
        Asked { owner: &self.owner, name: &self.name, number }
    }

    pub(super) fn asked_after<'a>(&'a self, number: u64, after: &'a str) -> AskedAfter<'a> {
        AskedAfter { owner: &self.owner, name: &self.name, number, after }
    }
}
