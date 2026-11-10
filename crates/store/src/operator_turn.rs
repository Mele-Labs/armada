//! The person's Your turn queue and the Operator's page state. Written by `fleet::operator`, whose
//! words the text columns hold.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One item that waits on the person, as it was raised.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptTurn {
    /// The row's number, once kept. Ignored as it is kept.
    pub n: i64,
    /// The Job whose list holds the item. Empty where no Job does.
    pub session: String,
    /// The item's id on its list.
    pub item: String,
    pub manifest_id: String,
    /// The Job's title.
    pub job: Option<String>,
    /// What the Operator would do itself, as against a decision that is the person's.
    pub go_ahead: bool,
    /// `ipc::WaitingItem`, as JSON.
    pub body: String,
    pub read_text: Option<String>,
    pub pick: Option<String>,
    /// Which Then the Operator picks, on a go-ahead to file an issue.
    pub then_pick: Option<String>,
    /// The order the person left it in. Assigned as it is kept.
    pub ord: i64,
    pub raised_at: String,
    /// Last in the queue until this instant.
    pub later_until: Option<String>,
    /// The last press on it.
    pub answer: Option<String>,
    pub settled_at: Option<String>,
    /// The call that read it.
    pub call: Option<String>,
}

/// What the page remembers between two looks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptPage {
    /// When Bridge's window was last in front.
    pub seen_at: Option<String>,
    /// When the person last looked: what the digest counts from.
    pub looked_at: Option<String>,
    /// The page's one line.
    pub top: Option<String>,
    pub top_at: Option<String>,
}

const COLUMNS: &str = "n, session, item, manifest_id, job, go_ahead, body, read_text, pick, then_pick, ord, raised_at, later_until, answer, settled_at, call";

fn row_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptTurn> {
    Ok(KeptTurn {
        n: row.get(0)?,
        session: row.get(1)?,
        item: row.get(2)?,
        manifest_id: row.get(3)?,
        job: row.get(4)?,
        go_ahead: row.get(5)?,
        body: row.get(6)?,
        read_text: row.get(7)?,
        pick: row.get(8)?,
        then_pick: row.get(9)?,
        ord: row.get(10)?,
        raised_at: row.get(11)?,
        later_until: row.get(12)?,
        answer: row.get(13)?,
        settled_at: row.get(14)?,
        call: row.get(15)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one item, last in the order. `None` where the same item is open already.
    pub fn add_operator_turn(&mut self, one: &KeptTurn) -> Result<Option<i64>, WriteError> {
        let doing = "keeping an item of the person's Your turn";
        let ord: i64 = self.conn.query_row("SELECT coalesce(max(ord), 0) + 1 FROM operator_turn", [], |row| row.get(0)).map_err(failed(doing))?;
        let kept = self.conn.execute(
            "INSERT OR IGNORE INTO operator_turn (session, item, manifest_id, job, go_ahead, body, read_text, pick, then_pick, ord, raised_at, later_until, answer, settled_at, call) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![one.session, one.item, one.manifest_id, one.job, one.go_ahead, one.body, one.read_text, one.pick, one.then_pick, ord, one.raised_at, one.later_until, one.answer, one.settled_at, one.call],
        );
        match kept.map_err(failed(doing))? {
            0 => Ok(None),
            _ => Ok(Some(self.conn.last_insert_rowid())),
        }
    }

    fn turn_read(&self, sql: &str, args: impl rusqlite::Params) -> Result<Vec<KeptTurn>, WriteError> {
        let doing = "reading the person's Your turn";
        let mut statement = self.conn.prepare(sql).map_err(failed(doing))?;
        let rows = statement.query_map(args, row_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Every item still waiting, in the order the person left them.
    pub fn operator_turn_open(&self) -> Result<Vec<KeptTurn>, WriteError> {
        self.turn_read(&format!("SELECT {COLUMNS} FROM operator_turn WHERE settled_at IS NULL ORDER BY ord, n"), params![])
    }

    /// Item `n`, open or settled.
    pub fn operator_turn_item(&self, n: i64) -> Result<Option<KeptTurn>, WriteError> {
        Ok(self.turn_read(&format!("SELECT {COLUMNS} FROM operator_turn WHERE n = ?1"), params![n])?.into_iter().next())
    }

    /// Every item settled at or after `since`, newest first.
    pub fn operator_turn_settled_since(&self, since: &str) -> Result<Vec<KeptTurn>, WriteError> {
        self.turn_read(&format!("SELECT {COLUMNS} FROM operator_turn WHERE settled_at >= ?1 ORDER BY settled_at DESC, n DESC"), params![since])
    }

    /// Item `n` settled at `at`, with the press that settled it. `false` where it was settled already.
    pub fn settle_operator_turn(&mut self, n: i64, at: &str, answer: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_turn SET settled_at = ?2, answer = ?3 WHERE n = ?1 AND settled_at IS NULL", params![n, at, answer])
            .map(|changed| changed > 0)
            .map_err(failed("settling an item of the person's Your turn"))
    }

    /// The press on open item `n` that leaves it open, such as a Yes that opens a walk.
    pub fn answer_operator_turn(&mut self, n: i64, answer: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_turn SET answer = ?2 WHERE n = ?1 AND settled_at IS NULL", params![n, answer])
            .map(|changed| changed > 0)
            .map_err(failed("keeping a press on an item of the person's Your turn"))
    }

    /// Open item `n` goes last until `until`.
    pub fn later_operator_turn(&mut self, n: i64, until: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_turn SET later_until = ?2 WHERE n = ?1 AND settled_at IS NULL", params![n, until])
            .map(|changed| changed > 0)
            .map_err(failed("putting an item of the person's Your turn last"))
    }

    /// The open items `ids` go first, in that order, and the rest after them as they were.
    pub fn order_operator_turn(&mut self, ids: &[i64]) -> Result<(), WriteError> {
        let doing = "ordering the person's Your turn";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let rest: Vec<i64> = {
            let mut statement = tx.prepare("SELECT n FROM operator_turn WHERE settled_at IS NULL ORDER BY ord, n").map_err(failed(doing))?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(failed(doing))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(failed(doing))?
        };
        let named: Vec<i64> = ids.iter().copied().filter(|id| rest.contains(id)).collect();
        let order = named.iter().chain(rest.iter().filter(|n| !named.contains(n)));
        for (at, n) in order.enumerate() {
            tx.execute("UPDATE operator_turn SET ord = ?2 WHERE n = ?1", params![n, i64::try_from(at).unwrap_or(0) + 1]).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))
    }

    /// What the page remembers.
    pub fn operator_page(&self) -> Result<KeptPage, WriteError> {
        let doing = "reading the Operator's page";
        let mut statement = self.conn.prepare("SELECT seen_at, looked_at, top, top_at FROM operator_page WHERE id = 1").map_err(failed(doing))?;
        let mut rows = statement
            .query_map([], |row| Ok(KeptPage { seen_at: row.get(0)?, looked_at: row.get(1)?, top: row.get(2)?, top_at: row.get(3)? }))
            .map_err(failed(doing))?;
        Ok(rows.next().transpose().map_err(failed(doing))?.unwrap_or_default())
    }

    /// Keep what the page remembers, whole.
    pub fn keep_operator_page(&mut self, page: &KeptPage) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO operator_page (id, seen_at, looked_at, top, top_at) VALUES (1, ?1, ?2, ?3, ?4) \
                 ON CONFLICT (id) DO UPDATE SET seen_at = ?1, looked_at = ?2, top = ?3, top_at = ?4",
                params![page.seen_at, page.looked_at, page.top, page.top_at],
            )
            .map(|_| ())
            .map_err(failed("keeping the Operator's page"))
    }
}
