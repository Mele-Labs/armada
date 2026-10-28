//! The refreshes of the preview Fleet offered on a Job's Needs you list, in place of starting one.
//!
//! **One row a (session, branch, tip, build)**, so a pair is offered once for a build and a Fleet
//! restart keeps that. Skip is the dismissed-ids list's, as for every item Fleet derives; a build
//! change deletes the rows of the old one.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One offer, as it was raised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPreviewOffer {
    pub session_id: String,
    pub branch: String,
    /// The branch's tip when it was offered, past the one it was left out at.
    pub tip: String,
    /// The commit of the build that left it out.
    pub build: String,
    pub text: String,
    pub raised_at: String,
}

impl Store {
    /// Keep `offer`. **True only where it is new**: one already kept for that (session, branch, tip,
    /// build) is left as it was.
    pub fn keep_preview_offer(&mut self, offer: &KeptPreviewOffer) -> Result<bool, WriteError> {
        let added = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO preview_offers (session_id, branch, tip, build, text, raised_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![offer.session_id, offer.branch, offer.tip, offer.build, offer.text, offer.raised_at],
            )
            .map_err(fault("keeping an offer to refresh the preview"))
            .map_err(WriteError::Database)?;
        Ok(added > 0)
    }

    /// The offers kept for `session_id`, by branch and then oldest first.
    pub fn preview_offers_of(&self, session_id: &str) -> Result<Vec<KeptPreviewOffer>, WriteError> {
        let doing = "reading the offers to refresh the preview";
        let mut statement = self
            .conn
            .prepare(
                "SELECT branch, tip, build, text, raised_at FROM preview_offers WHERE session_id = ?1 ORDER BY branch, rowid",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptPreviewOffer {
                    session_id: session_id.to_string(),
                    branch: row.get(0)?,
                    tip: row.get(1)?,
                    build: row.get(2)?,
                    text: row.get(3)?,
                    raised_at: row.get(4)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Delete every offer made for a build other than `build`, or every offer where none runs. Answers
    /// the sessions that lost one, each once.
    pub fn drop_preview_offers_not_for(&mut self, build: Option<&str>) -> Result<Vec<String>, WriteError> {
        let doing = "dropping offers to refresh an old preview";
        let build = build.unwrap_or_default();
        let mut statement = self
            .conn
            .prepare("DELETE FROM preview_offers WHERE build != ?1 RETURNING session_id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut sessions = statement
            .query_map([build], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        sessions.sort();
        sessions.dedup();
        Ok(sessions)
    }
}
