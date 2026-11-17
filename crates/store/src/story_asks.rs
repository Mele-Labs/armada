//! What the owner asked of a Job, sorted from his messages: the asks, the kind each message was sorted as, the
//! pull requests that built an ask, the hands from one Job to another, and the reply that answers each question of his.
//! `docs/concepts/job-story.md`.
//!
//! **Strings here, words in `ipc`.** The store holds kinds and states as the table spells them, and Fleet reads
//! them into the wire's types. A row it cannot read is Fleet's to leave out.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// An ask's state, as the table spells it.
pub mod ask_state {
    pub const BACKLOG: &str = "backlog";
    pub const BUILDING: &str = "building";
    pub const BUILT: &str = "built";
    pub const HANDED: &str = "handed";
    pub const DROPPED: &str = "dropped";
}

/// What a message of his was sorted as.
pub mod message_kind {
    pub const NEW_ASK: &str = "new_ask";
    pub const REVISION: &str = "revision";
    pub const ANSWER: &str = "answer";
    pub const QUESTION: &str = "question";
    pub const TALK: &str = "talk";
    /// Delivered and waiting for the model.
    pub const UNSORTED: &str = "unsorted";
    /// Taken by a read of the model, which keeps its answer or drops the row.
    pub const SORTING: &str = "sorting";
}

/// Who sorted a message.
pub mod sorted_by {
    pub const FLEET: &str = "fleet";
    pub const OWNER: &str = "owner";
}

/// One ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAsk {
    pub ask_id: String,
    /// The Job it was said in.
    pub session_id: String,
    /// The repository and project that Job was in.
    pub manifest_id: Option<String>,
    pub project: Option<String>,
    /// `bug`, `change_request`, `idea` or `fix`.
    pub kind: String,
    pub title: String,
    pub state: String,
    /// The Job building it, where one is.
    pub building: Option<String>,
    pub opened_at: String,
    /// When `state` last moved.
    pub state_at: String,
}

/// One message of his, as it was sorted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMessage {
    pub session_id: String,
    pub row_id: String,
    pub kind: String,
    /// The ask it opened, revised or answered. Absent for talk.
    pub ask_id: Option<String>,
    pub words: String,
    /// When he said it.
    pub at: String,
    pub sorted_by: String,
    pub sorted_at: String,
}

/// A message he sorted himself, with the kind of the ask it opened where it opened one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCorrection {
    pub words: String,
    pub kind: String,
    pub ask_kind: Option<String>,
}

/// A pull request that built an ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptMerge {
    pub ask_id: String,
    pub pull_request: String,
    pub merged_at: String,
}

/// An ask passed from one Job to another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHandoff {
    pub ask_id: String,
    pub from_session: String,
    pub to_session: String,
    pub at: String,
}

/// A question of his, with the reply that answers it where one does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptQuestion {
    pub session_id: String,
    pub row_id: String,
    /// His words and when he said them, as `story_messages` holds them.
    pub words: String,
    pub at: String,
    /// The last reply read as not answering it.
    pub checked_row: Option<String>,
    /// The agent's message row that answers it, its excerpt and when it was read as answering.
    pub reply_row: Option<String>,
    pub excerpt: Option<String>,
    pub answered_at: Option<String>,
    /// When he read the answer.
    pub read_at: Option<String>,
}

const QUESTION_COLUMNS: &str = "q.session_id, q.row_id, m.words, m.at, q.checked_row, q.reply_row, q.excerpt, q.answered_at, q.read_at";
const QUESTION_JOIN: &str = "story_questions q JOIN story_messages m ON m.session_id = q.session_id AND m.row_id = q.row_id AND m.kind = 'question'";

fn question_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptQuestion> {
    Ok(KeptQuestion {
        session_id: row.get(0)?,
        row_id: row.get(1)?,
        words: row.get(2)?,
        at: row.get(3)?,
        checked_row: row.get(4)?,
        reply_row: row.get(5)?,
        excerpt: row.get(6)?,
        answered_at: row.get(7)?,
        read_at: row.get(8)?,
    })
}

const ASK_COLUMNS: &str = "ask_id, session_id, manifest_id, project, kind, title, state, building, opened_at, state_at";
const MESSAGE_COLUMNS: &str = "session_id, row_id, kind, ask_id, words, at, sorted_by, sorted_at";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn ask_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptAsk> {
    Ok(KeptAsk {
        ask_id: row.get(0)?,
        session_id: row.get(1)?,
        manifest_id: row.get(2)?,
        project: row.get(3)?,
        kind: row.get(4)?,
        title: row.get(5)?,
        state: row.get(6)?,
        building: row.get(7)?,
        opened_at: row.get(8)?,
        state_at: row.get(9)?,
    })
}

fn message_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptMessage> {
    Ok(KeptMessage {
        session_id: row.get(0)?,
        row_id: row.get(1)?,
        kind: row.get(2)?,
        ask_id: row.get(3)?,
        words: row.get(4)?,
        at: row.get(5)?,
        sorted_by: row.get(6)?,
        sorted_at: row.get(7)?,
    })
}

impl Store {
    /// Keep `ask`, replacing what the one of its id says of its kind, title, state and builder. Where it was said and when stay as first kept.
    pub fn keep_ask(&mut self, ask: &KeptAsk) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO story_asks ({ASK_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                     ON CONFLICT (ask_id) DO UPDATE SET kind = ?5, title = ?6, state = ?7, building = ?8, state_at = ?10"
                ),
                params![
                    ask.ask_id,
                    ask.session_id,
                    ask.manifest_id,
                    ask.project,
                    ask.kind,
                    ask.title,
                    ask.state,
                    ask.building,
                    ask.opened_at,
                    ask.state_at
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping an ask"))
    }

    /// The ask `ask_id`.
    pub fn ask(&self, ask_id: &str) -> Result<Option<KeptAsk>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {ASK_COLUMNS} FROM story_asks WHERE ask_id = ?1"), [ask_id], ask_of)
            .optional()
            .map_err(failed("reading an ask"))
    }

    fn asks_where(&self, doing: &'static str, clause: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<KeptAsk>, WriteError> {
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {ASK_COLUMNS} FROM story_asks WHERE {clause} ORDER BY opened_at, ask_id"))
            .map_err(failed(doing))?;
        let rows = statement.query_map(args, ask_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The asks said in `session` or built by it, oldest first.
    pub fn asks_of_session(&self, session: &str) -> Result<Vec<KeptAsk>, WriteError> {
        self.asks_where(
            "reading the asks of a Job",
            "session_id = ?1 OR building = ?1 OR ask_id IN (SELECT ask_id FROM story_ask_handoffs WHERE from_session = ?1 OR to_session = ?1)",
            &[&session],
        )
    }

    /// The asks of `project`, or of the repository of that name, oldest first. `feature` narrows them to those filed under that slug.
    pub fn asks_of_project(&self, project: &str, feature: Option<&str>) -> Result<Vec<KeptAsk>, WriteError> {
        match feature {
            Some(slug) => self.asks_where(
                "reading the asks of a project's feature",
                "(project = ?1 OR manifest_id = ?1) AND ask_id IN (SELECT ask_id FROM story_ask_features WHERE slug = ?2)",
                &[&project, &slug],
            ),
            None => self.asks_where("reading the asks of a project", "project = ?1 OR manifest_id = ?1", &[&project]),
        }
    }

    /// The asks a Job is building: `building` or `handed`, whose pull requests are read for a merge.
    pub fn asks_in_work(&self) -> Result<Vec<KeptAsk>, WriteError> {
        self.asks_where(
            "reading the asks in work",
            "state IN ('building', 'handed') AND building IS NOT NULL",
            &[],
        )
    }

    /// Remove the ask and everything kept on it. The messages that joined it stay, as talk.
    pub fn remove_ask(&mut self, ask_id: &str) -> Result<(), WriteError> {
        let doing = "removing an ask";
        for table in ["story_ask_features", "story_ask_merges", "story_ask_handoffs"] {
            self.conn.execute(&format!("DELETE FROM {table} WHERE ask_id = ?1"), [ask_id]).map_err(failed(doing))?;
        }
        self.conn.execute("UPDATE story_messages SET ask_id = NULL, kind = 'talk' WHERE ask_id = ?1", [ask_id]).map_err(failed(doing))?;
        self.conn.execute("DELETE FROM story_asks WHERE ask_id = ?1", [ask_id]).map(|_| ()).map_err(failed(doing))
    }

    /// Replace the feature slugs `ask_id` is filed under.
    pub fn keep_ask_features(&mut self, ask_id: &str, slugs: &[String]) -> Result<(), WriteError> {
        let doing = "filing an ask under features";
        self.conn.execute("DELETE FROM story_ask_features WHERE ask_id = ?1", [ask_id]).map_err(failed(doing))?;
        for slug in slugs {
            self.conn
                .execute("INSERT OR IGNORE INTO story_ask_features (ask_id, slug) VALUES (?1, ?2)", params![ask_id, slug])
                .map_err(failed(doing))?;
        }
        Ok(())
    }

    /// The slugs `ask_id` is filed under, in the order they sort.
    pub fn features_of_ask(&self, ask_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading the features of an ask";
        let mut statement = self.conn.prepare("SELECT slug FROM story_ask_features WHERE ask_id = ?1 ORDER BY slug").map_err(failed(doing))?;
        let rows = statement.query_map([ask_id], |row| row.get(0)).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep that `pull_request` merged for `ask_id`, **once**: whether this call added it.
    pub fn keep_ask_merge(&mut self, merge: &KeptMerge) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO story_ask_merges (ask_id, pull_request, merged_at) VALUES (?1, ?2, ?3)",
                params![merge.ask_id, merge.pull_request, merge.merged_at],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping a merge of an ask"))
    }

    /// The pull requests that built `ask_id`, oldest merge first.
    pub fn merges_of_ask(&self, ask_id: &str) -> Result<Vec<KeptMerge>, WriteError> {
        let doing = "reading the merges of an ask";
        let mut statement = self
            .conn
            .prepare("SELECT ask_id, pull_request, merged_at FROM story_ask_merges WHERE ask_id = ?1 ORDER BY merged_at, pull_request")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([ask_id], |row| Ok(KeptMerge { ask_id: row.get(0)?, pull_request: row.get(1)?, merged_at: row.get(2)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep that `ask_id` went from one Job to another, **once a Job**.
    pub fn keep_ask_handoff(&mut self, hand: &KeptHandoff) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO story_ask_handoffs (ask_id, from_session, to_session, at) VALUES (?1, ?2, ?3, ?4)",
                params![hand.ask_id, hand.from_session, hand.to_session, hand.at],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping a hand of an ask"))
    }

    /// The hands of `ask_id`, oldest first.
    pub fn handoffs_of_ask(&self, ask_id: &str) -> Result<Vec<KeptHandoff>, WriteError> {
        let doing = "reading the hands of an ask";
        let mut statement = self
            .conn
            .prepare("SELECT ask_id, from_session, to_session, at FROM story_ask_handoffs WHERE ask_id = ?1 ORDER BY at, to_session")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([ask_id], |row| {
                Ok(KeptHandoff { ask_id: row.get(0)?, from_session: row.get(1)?, to_session: row.get(2)?, at: row.get(3)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep how a message was sorted, replacing the sorting of the message of that Job and row.
    pub fn keep_message(&mut self, message: &KeptMessage) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO story_messages ({MESSAGE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                     ON CONFLICT (session_id, row_id) DO UPDATE SET kind = ?3, ask_id = ?4, words = ?5, sorted_by = ?7, sorted_at = ?8"
                ),
                params![
                    message.session_id,
                    message.row_id,
                    message.kind,
                    message.ask_id,
                    message.words,
                    message.at,
                    message.sorted_by,
                    message.sorted_at
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping how a message was sorted"))
    }

    /// How the message `row_id` of `session` was sorted, where it was.
    pub fn message_sorted(&self, session: &str, row_id: &str) -> Result<Option<KeptMessage>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {MESSAGE_COLUMNS} FROM story_messages WHERE session_id = ?1 AND row_id = ?2"),
                params![session, row_id],
                message_of,
            )
            .optional()
            .map_err(failed("reading how a message was sorted"))
    }

    /// The messages of `session` that were sorted, oldest first.
    pub fn messages_sorted_in(&self, session: &str) -> Result<Vec<KeptMessage>, WriteError> {
        let doing = "reading how a Job's messages were sorted";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM story_messages WHERE session_id = ?1 AND kind IN ('new_ask', 'revision', 'answer', 'question', 'talk') ORDER BY at, row_id"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([session], message_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The messages that opened, revised or answered `ask_id`, oldest first.
    pub fn messages_of_ask(&self, ask_id: &str) -> Result<Vec<KeptMessage>, WriteError> {
        let doing = "reading the words of an ask";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM story_messages WHERE ask_id = ?1 ORDER BY at, row_id"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([ask_id], message_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// A message of his that was delivered, to be sorted. **Once a message**: one already kept, sorted or not, stays as it is.
    pub fn keep_message_unsorted(&mut self, message: &KeptMessage) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR IGNORE INTO story_messages ({MESSAGE_COLUMNS}) VALUES (?1, ?2, 'unsorted', NULL, ?3, ?4, 'fleet', ?5)"),
                params![message.session_id, message.row_id, message.words, message.at, message.sorted_at],
            )
            .map(|_| ())
            .map_err(failed("keeping a message to be sorted"))
    }

    /// Take the oldest `limit` messages waiting to be sorted, marking them `sorting` at `at`, so no second read takes them.
    pub fn take_unsorted(&mut self, limit: u32, at: &str) -> Result<Vec<KeptMessage>, WriteError> {
        let doing = "taking the messages to be sorted";
        let taken: Vec<KeptMessage> = {
            let mut statement = self
                .conn
                .prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM story_messages WHERE kind = 'unsorted' ORDER BY at, row_id LIMIT ?1"))
                .map_err(failed(doing))?;
            let rows = statement.query_map([limit], message_of).map_err(failed(doing))?;
            rows.collect::<Result<_, _>>().map_err(failed(doing))?
        };
        for one in &taken {
            self.conn
                .execute(
                    "UPDATE story_messages SET kind = 'sorting', sorted_at = ?3 WHERE session_id = ?1 AND row_id = ?2 AND kind = 'unsorted'",
                    params![one.session_id, one.row_id, at],
                )
                .map_err(failed(doing))?;
        }
        Ok(taken)
    }

    /// Keep what the model made of a message taken by `take_unsorted`, **only while it is still `sorting`**: whether it was kept.
    /// A message he sorted himself meanwhile is his.
    pub fn keep_message_read(&mut self, message: &KeptMessage) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE story_messages SET kind = ?3, ask_id = ?4, sorted_by = 'fleet', sorted_at = ?5 \
                 WHERE session_id = ?1 AND row_id = ?2 AND kind = 'sorting'",
                params![message.session_id, message.row_id, message.kind, message.ask_id, message.sorted_at],
            )
            .map(|moved| moved > 0)
            .map_err(failed("keeping what the model made of a message"))
    }

    /// Let a message taken by `take_unsorted` go unsorted: the read failed, so the thread draws no chip for it.
    pub fn drop_message_read(&mut self, session: &str, row_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM story_messages WHERE session_id = ?1 AND row_id = ?2 AND kind = 'sorting'", params![session, row_id])
            .map(|_| ())
            .map_err(failed("letting a message go unsorted"))
    }

    /// The words of a message he edited, where it was sorted.
    pub fn keep_message_words(&mut self, session: &str, row_id: &str, words: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE story_messages SET words = ?3 WHERE session_id = ?1 AND row_id = ?2", params![session, row_id, words])
            .map(|_| ())
            .map_err(failed("keeping the words of an edited message"))
    }

    /// The newest `limit` messages he sorted himself, newest first.
    pub fn corrections(&self, limit: u32) -> Result<Vec<KeptCorrection>, WriteError> {
        let doing = "reading the corrections of his sorting";
        let mut statement = self
            .conn
            .prepare(
                "SELECT m.words, m.kind, a.kind FROM story_messages m \
                 LEFT JOIN story_asks a ON a.ask_id = m.ask_id AND m.kind = 'new_ask' \
                 WHERE m.sorted_by = 'owner' ORDER BY m.sorted_at DESC, m.row_id DESC LIMIT ?1",
            )
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([limit], |row| Ok(KeptCorrection { words: row.get(0)?, kind: row.get(1)?, ask_kind: row.get(2)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The message `row_id` of `session` is a question, **due a read**. One already kept keeps its answer.
    pub fn keep_question(&mut self, session: &str, row_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO story_questions (session_id, row_id, due) VALUES (?1, ?2, 1)", params![session, row_id])
            .map(|_| ())
            .map_err(failed("keeping a question"))
    }

    /// The message is no longer a question: it and its answer go.
    pub fn remove_question(&mut self, session: &str, row_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM story_questions WHERE session_id = ?1 AND row_id = ?2", params![session, row_id])
            .map(|_| ())
            .map_err(failed("removing a question"))
    }

    /// The Job's turn ended: each question of `session` still unanswered is due a read. Whether any is.
    pub fn questions_due(&mut self, session: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE story_questions SET due = 1 WHERE session_id = ?1 AND reply_row IS NULL", [session])
            .map(|moved| moved > 0)
            .map_err(failed("marking a Job's questions due"))
    }

    /// The question `row_id` of `session` is due a read again, while it is unanswered.
    pub fn keep_question_due(&mut self, session: &str, row_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE story_questions SET due = 1 WHERE session_id = ?1 AND row_id = ?2 AND reply_row IS NULL", params![session, row_id])
            .map(|_| ())
            .map_err(failed("marking a question due again"))
    }

    /// Take the oldest `limit` questions due a read, so no second read takes them. One whose read fails is marked due again by Fleet, a few times.
    pub fn take_due_questions(&mut self, limit: u32) -> Result<Vec<KeptQuestion>, WriteError> {
        let doing = "taking the questions due a read";
        let taken: Vec<KeptQuestion> = {
            let mut statement = self
                .conn
                .prepare(&format!("SELECT {QUESTION_COLUMNS} FROM {QUESTION_JOIN} WHERE q.due = 1 AND q.reply_row IS NULL ORDER BY m.at, q.row_id LIMIT ?1"))
                .map_err(failed(doing))?;
            let rows = statement.query_map([limit], question_of).map_err(failed(doing))?;
            rows.collect::<Result<_, _>>().map_err(failed(doing))?
        };
        for one in &taken {
            self.conn
                .execute("UPDATE story_questions SET due = 0 WHERE session_id = ?1 AND row_id = ?2", params![one.session_id, one.row_id])
                .map_err(failed(doing))?;
        }
        Ok(taken)
    }

    /// The reply `checked_row` was read as not answering the question.
    pub fn keep_question_checked(&mut self, session: &str, row_id: &str, checked_row: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE story_questions SET checked_row = ?3 WHERE session_id = ?1 AND row_id = ?2 AND reply_row IS NULL",
                params![session, row_id, checked_row],
            )
            .map(|_| ())
            .map_err(failed("keeping a reply read as not answering"))
    }

    /// The reply `reply_row` answers the question, unread. **Once**: whether this call linked it.
    pub fn keep_question_answered(&mut self, session: &str, row_id: &str, reply_row: &str, excerpt: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE story_questions SET reply_row = ?3, excerpt = ?4, answered_at = ?5, read_at = NULL, due = 0 \
                 WHERE session_id = ?1 AND row_id = ?2 AND reply_row IS NULL",
                params![session, row_id, reply_row, excerpt, at],
            )
            .map(|moved| moved > 0)
            .map_err(failed("keeping the reply that answers a question"))
    }

    /// He read the answer to the question, at `at`. Whether it was unread until now.
    pub fn keep_answer_read(&mut self, session: &str, row_id: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE story_questions SET read_at = ?3 WHERE session_id = ?1 AND row_id = ?2 AND reply_row IS NOT NULL AND read_at IS NULL",
                params![session, row_id, at],
            )
            .map(|moved| moved > 0)
            .map_err(failed("keeping an answer read"))
    }

    /// The question `row_id` of `session`, where the message is one.
    pub fn question(&self, session: &str, row_id: &str) -> Result<Option<KeptQuestion>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {QUESTION_COLUMNS} FROM {QUESTION_JOIN} WHERE q.session_id = ?1 AND q.row_id = ?2"), params![session, row_id], question_of)
            .optional()
            .map_err(failed("reading a question"))
    }

    /// The questions of `session`: the unanswered first, each group newest first.
    pub fn questions_of(&self, session: &str) -> Result<Vec<KeptQuestion>, WriteError> {
        let doing = "reading the questions of a Job";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {QUESTION_COLUMNS} FROM {QUESTION_JOIN} WHERE q.session_id = ?1 ORDER BY q.reply_row IS NOT NULL, m.at DESC, q.row_id DESC"
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map([session], question_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Whether a question of `session` has an answer he has not read.
    pub fn answer_unread(&self, session: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT EXISTS (SELECT 1 FROM {QUESTION_JOIN} WHERE q.session_id = ?1 AND q.reply_row IS NOT NULL AND q.read_at IS NULL)"),
                [session],
                |row| row.get(0),
            )
            .map_err(failed("reading whether an answer waits unread"))
    }
}
