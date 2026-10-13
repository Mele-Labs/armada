//! The card a Job's Commander has put to the owner. `#2166`.
//!
//! **Kept, not held.** A Drone's question is a column and waits however long a
//! person takes; a card is the same shape in a table of its own, so a Fleet that
//! restarts still has it and nothing times it out. A Job has one out at a time
//! (the unique index), and an answered card stays as history.

use core_model::{JobId, Timestamp};
use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, DatabaseFault, RowError, WriteError};
use crate::open::Store;

/// Who put the card out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardRaiser {
    /// The Commander, through `ask_the_owner`.
    Commander,
    /// Fleet, where the loop bound stopped the Commander acting on a problem.
    Fleet,
}

impl CardRaiser {
    pub fn as_wire(self) -> &'static str {
        match self {
            CardRaiser::Commander => "commander",
            CardRaiser::Fleet => "fleet",
        }
    }

    fn from_wire(word: &str) -> Option<CardRaiser> {
        match word {
            "commander" => Some(CardRaiser::Commander),
            "fleet" => Some(CardRaiser::Fleet),
            _ => None,
        }
    }
}

/// A card to be written. The options are labels in the order asked; an option's
/// id is its one-based place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCard {
    pub card_id: String,
    pub raised_by: CardRaiser,
    pub raised_at: Timestamp,
    pub text: String,
    pub reach: String,
    pub question: String,
    pub options: Vec<String>,
}

/// A card as stored. `chose` is the id of the option he picked and is `None`
/// while the card is out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCard {
    pub card_id: String,
    pub job_id: JobId,
    pub raised_by: CardRaiser,
    pub raised_at: Timestamp,
    pub text: String,
    pub reach: String,
    pub question: String,
    pub options: Vec<String>,
    pub chose: Option<String>,
    pub answered_at: Option<Timestamp>,
}

impl StoredCard {
    /// The label of an option, by id. `None` for an id the card never offered.
    pub fn label_of(&self, option_id: &str) -> Option<&str> {
        let place: usize = option_id.parse().ok()?;
        self.options.get(place.checked_sub(1)?).map(String::as_str)
    }
}

/// What raising a card found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardRaised {
    Out,
    /// The Job already has one out, which is returned and left as it was.
    AlreadyOut(Box<StoredCard>),
}

/// What answering a card found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardAnswered {
    /// Recorded. The card as it stands now, answered.
    Recorded(Box<StoredCard>),
    /// That card is not the one out: answered already, or never this Job's.
    NotOut,
    /// The card is out and did not offer this option.
    NoSuchOption(Box<StoredCard>),
}

const COLUMNS: &str =
    "card_id, job_id, raised_by, raised_at, text, reach, question, options, chose, answered_at";

fn card_of(row: &Row<'_>) -> Result<StoredCard, RowError> {
    let malformed = |column: &'static str, detail: &str| RowError::MalformedColumn {
        table: "commander_cards",
        column,
        detail: detail.to_string(),
    };
    let database = |why| RowError::Database(fault("reading a Commander's card")(why));
    let raised_by: String = row.get(2).map_err(database)?;
    let options: String = row.get(7).map_err(database)?;
    let answered_at: Option<String> = row.get(9).map_err(database)?;
    Ok(StoredCard {
        card_id: row.get(0).map_err(database)?,
        job_id: JobId::carried(core_model::Ulid::carried(
            row.get::<_, String>(1).map_err(database)?,
        )),
        raised_by: CardRaiser::from_wire(&raised_by)
            .ok_or_else(|| malformed("raised_by", "neither commander nor fleet"))?,
        raised_at: Timestamp::from_rfc3339(row.get::<_, String>(3).map_err(database)?),
        text: row.get(4).map_err(database)?,
        reach: row.get(5).map_err(database)?,
        question: row.get(6).map_err(database)?,
        options: serde_json::from_str(&options)
            .map_err(|why| malformed("options", &why.to_string()))?,
        chose: row.get(8).map_err(database)?,
        answered_at: answered_at.map(Timestamp::from_rfc3339),
    })
}

/// A row that would not read as a card, as the write side's one error.
fn unreadable(why: RowError) -> WriteError {
    WriteError::Database(DatabaseFault {
        doing: "reading a Commander's card",
        cause: Box::new(std::io::Error::other(why.to_string())),
    })
}

impl Store {
    /// Put a card out for a Job, unless it has one out already. **The Job's
    /// existing card is not touched**: the caller is told which, so the
    /// Commander reads what it is waiting on.
    pub fn raise_commander_card(
        &mut self,
        job_id: &JobId,
        card: &NewCard,
    ) -> Result<CardRaised, WriteError> {
        if let Some(out) = self.open_commander_card(job_id)? {
            return Ok(CardRaised::AlreadyOut(Box::new(out)));
        }
        let options = serde_json::to_string(&card.options).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                "INSERT INTO commander_cards \
                 (card_id, job_id, raised_by, raised_at, text, reach, question, options) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &card.card_id,
                    job_id.as_str(),
                    card.raised_by.as_wire(),
                    card.raised_at.as_str(),
                    &card.text,
                    &card.reach,
                    &card.question,
                    options,
                ),
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("putting a Commander's card out")(other)),
            })?;
        Ok(CardRaised::Out)
    }

    /// The card this Job has out, if any.
    pub fn open_commander_card(&self, job_id: &JobId) -> Result<Option<StoredCard>, WriteError> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM commander_cards \
                     WHERE job_id = ?1 AND answered_at IS NULL"
                ),
                (job_id.as_str(),),
                |row| Ok(card_of(row)),
            )
            .optional()
            .map_err(fault("reading a Commander's card"))
            .map_err(WriteError::Database)?
            .transpose()
            .map_err(unreadable)
    }

    /// Every card out, for every Job: what the night's sweep reads.
    pub fn open_commander_cards(&self) -> Result<Vec<StoredCard>, WriteError> {
        let doing = "reading the cards out";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM commander_cards WHERE answered_at IS NULL \
                 ORDER BY raised_at, card_id"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(card_of(row)))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut cards = Vec::new();
        for row in rows {
            let row = row.map_err(fault(doing)).map_err(WriteError::Database)?;
            cards.push(row.map_err(unreadable)?);
        }
        Ok(cards)
    }

    /// Record his answer: the option's id, by the card's id. **Only the card
    /// out can be answered**, so an answer sent from a window left open across
    /// a newer card is refused rather than read against the wrong options.
    pub fn answer_commander_card(
        &mut self,
        job_id: &JobId,
        card_id: &str,
        option_id: &str,
        at: &Timestamp,
    ) -> Result<CardAnswered, WriteError> {
        let out = self.open_commander_card(job_id)?;
        let Some(out) = out.filter(|card| card.card_id == card_id) else {
            return Ok(CardAnswered::NotOut);
        };
        if out.label_of(option_id).is_none() {
            return Ok(CardAnswered::NoSuchOption(Box::new(out)));
        }
        self.conn
            .execute(
                "UPDATE commander_cards SET chose = ?2, answered_at = ?3 \
                 WHERE card_id = ?1 AND answered_at IS NULL",
                (card_id, option_id, at.as_str()),
            )
            .map_err(fault("recording the answer to a Commander's card"))
            .map_err(WriteError::Database)?;
        Ok(CardAnswered::Recorded(Box::new(StoredCard {
            chose: Some(option_id.to_string()),
            answered_at: Some(at.clone()),
            ..out
        })))
    }
}
