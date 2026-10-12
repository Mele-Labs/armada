//! The question a Job's Drone asked, and the answer until the next Drone opens
//! with it. `core_model::DroneQuestion` has why it is on the record.
//!
//! **One writer for every column**, for `crate::pausing`'s reason: a question
//! half written reads as a different question.

use core_model::{DroneQuestion, GivenAnswer, Job, OfferedAnswer, StepId, Timestamp, Ulid};
use rusqlite::Row;

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::row::maybe;

fn malformed(column: &'static str, detail: &str) -> RowError {
    RowError::MalformedColumn {
        table: "jobs",
        column,
        detail: detail.to_string(),
    }
}

/// The question as the row holds it, set on `job` where there is one.
pub(crate) fn read_question(row: &Row<'_>, job: Job) -> Result<Job, RowError> {
    let Some(id) = maybe(row, "question_id")? else {
        return Ok(job);
    };
    let held = |column: &'static str| {
        maybe(row, column)?.ok_or_else(|| malformed(column, "a question without it"))
    };
    let options: Vec<(String, String)> = serde_json::from_str(&held("question_options")?)
        .map_err(|why| malformed("question_options", &why.to_string()))?;
    let answer = match (
        maybe(row, "question_chose")?,
        maybe(row, "question_deciding")?,
        maybe(row, "question_answered_at")?,
    ) {
        (label, deciding, Some(at)) => Some(GivenAnswer {
            label,
            deciding,
            at: Timestamp::from_rfc3339(at),
        }),
        (None, None, None) => None,
        _ => return Err(malformed("question_answered_at", "an answer with no instant")),
    };
    Ok(job.asking(DroneQuestion {
        id: Ulid::carried(id),
        step: StepId::new(held("question_step")?),
        asked_at: Timestamp::from_rfc3339(held("question_asked_at")?),
        question: held("question_text")?,
        options: options
            .into_iter()
            .map(|(label, consequence)| OfferedAnswer { label, consequence })
            .collect(),
        answer,
    }))
}

impl Store {
    /// Write the Job's question as it stands: asked, answered, or gone. No
    /// event, for `record_pause`'s reason.
    pub fn record_question(&mut self, job: &Job) -> Result<(), WriteError> {
        let question = job.question();
        let options = question.map(|question| {
            let pairs: Vec<(&str, &str)> = question
                .options
                .iter()
                .map(|option| (option.label.as_str(), option.consequence.as_str()))
                .collect();
            serde_json::to_string(&pairs).unwrap_or_else(|_| String::from("[]"))
        });
        let answer = question.and_then(|question| question.answer.as_ref());
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET question_id = ?2, question_step = ?3, question_asked_at = ?4,
                        question_text = ?5, question_options = ?6, question_chose = ?7,
                        question_answered_at = ?8, question_deciding = ?9
                 WHERE job_id = ?1",
                (
                    job.id().as_str(),
                    question.map(|question| question.id.as_str()),
                    question.map(|question| question.step.as_str()),
                    question.map(|question| question.asked_at.as_str()),
                    question.map(|question| question.question.as_str()),
                    options,
                    answer.and_then(|given| given.label.as_deref()),
                    answer.map(|given| given.at.as_str()),
                    answer.and_then(|given| given.deciding.as_deref()),
                ),
            )
            .map_err(fault("recording a drone's question"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
    }
}
