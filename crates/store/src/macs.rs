//! The owner's other Macs, as the Phone Gateway pairs and links them. Times are Unix seconds, read
//! by the caller from its own clock. `docs/concepts/pocket.md`, *Another Mac*.

use crate::error::{fault, DatabaseFault};
use crate::open::Store;

/// Where a pairing with another Mac stands, as this Mac keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacState {
    /// This Mac claimed the other's link and waits for Confirm there.
    ConfirmWaiting,
    /// Both sides hold a key for the other.
    Linked,
}

impl MacState {
    fn name(self) -> &'static str {
        match self {
            MacState::ConfirmWaiting => "confirm_waiting",
            MacState::Linked => "linked",
        }
    }

    fn named(name: &str) -> Option<MacState> {
        match name {
            "confirm_waiting" => Some(MacState::ConfirmWaiting),
            "linked" => Some(MacState::Linked),
            _ => None,
        }
    }
}

/// Another Mac, as it said it was when it was last heard from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedMac {
    pub id: String,
    pub name: String,
    pub colour: String,
    pub address: String,
    /// SPKI DER of the key its calls are signed with. `None` until it confirms.
    pub public_key: Option<Vec<u8>>,
    pub build: Option<String>,
    pub paired_at: i64,
    pub state: MacState,
    /// The one-time code this Mac minted for the other's Confirm to bring back.
    pub back_code: Option<String>,
    pub last_heard_at: Option<i64>,
    pub asleep_since: Option<i64>,
    pub why: Option<String>,
}

/// What the other Mac says of itself each time it is heard from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacSaid {
    pub name: String,
    pub colour: String,
    pub address: String,
    pub build: Option<String>,
}

const COLUMNS: &str =
    "mac_id, name, colour, address, public_key, build, paired_at, state, back_code, last_heard_at, asleep_since, why";

fn paired_mac(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<PairedMac>> {
    let state: String = row.get(7)?;
    // A state a newer build wrote is left out rather than failing the list.
    let Some(state) = MacState::named(&state) else { return Ok(None) };
    Ok(Some(PairedMac {
        id: row.get(0)?,
        name: row.get(1)?,
        colour: row.get(2)?,
        address: row.get(3)?,
        public_key: row.get(4)?,
        build: row.get(5)?,
        paired_at: row.get(6)?,
        state,
        back_code: row.get(8)?,
        last_heard_at: row.get(9)?,
        asleep_since: row.get(10)?,
        why: row.get(11)?,
    }))
}

impl Store {
    /// Keep `mac`, in place of any row it had: pairing again starts it afresh.
    pub fn keep_mac(&mut self, mac: &PairedMac) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO pocket_macs ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
                rusqlite::params![
                    mac.id,
                    mac.name,
                    mac.colour,
                    mac.address,
                    mac.public_key,
                    mac.build,
                    mac.paired_at,
                    mac.state.name(),
                    mac.back_code,
                    mac.last_heard_at,
                    mac.asleep_since,
                    mac.why,
                ],
            )
            .map_err(fault("pairing another Mac"))?;
        Ok(())
    }

    pub fn paired_mac(&self, id: &str) -> Result<Option<PairedMac>, DatabaseFault> {
        let doing = "reading a paired Mac";
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM pocket_macs WHERE mac_id = ?1"))
            .map_err(fault(doing))?;
        let mut rows = asking.query_map((id,), paired_mac).map_err(fault(doing))?;
        Ok(rows.next().transpose().map_err(fault(doing))?.flatten())
    }

    /// Every other Mac, in the order they were paired.
    pub fn paired_macs(&self) -> Result<Vec<PairedMac>, DatabaseFault> {
        let doing = "reading the paired Macs";
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM pocket_macs ORDER BY paired_at, mac_id"))
            .map_err(fault(doing))?;
        let rows = asking.query_map([], paired_mac).map_err(fault(doing))?;
        let all: Vec<Option<PairedMac>> = rows.collect::<Result<_, _>>().map_err(fault(doing))?;
        Ok(all.into_iter().flatten().collect())
    }

    /// The other Mac confirmed: it signs with `public_key` from now on, and its code is spent.
    pub fn link_mac(&mut self, id: &str, public_key: &[u8], said: &MacSaid) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "UPDATE pocket_macs SET public_key = ?2, state = 'linked', back_code = NULL, \
                 name = ?3, colour = ?4, address = ?5, build = ?6 WHERE mac_id = ?1",
                (id, public_key, &said.name, &said.colour, &said.address, &said.build),
            )
            .map_err(fault("linking another Mac"))?;
        Ok(())
    }

    /// The other Mac answered at `at`: it is awake, and says who it is now.
    pub fn heard_from_mac(&mut self, id: &str, at: i64, said: &MacSaid) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "UPDATE pocket_macs SET last_heard_at = ?2, asleep_since = NULL, why = NULL, \
                 name = ?3, colour = ?4, address = ?5, build = ?6 WHERE mac_id = ?1",
                (id, at, &said.name, &said.colour, &said.address, &said.build),
            )
            .map_err(fault("noting another Mac answered"))?;
        Ok(())
    }

    /// Something arrived from the other Mac at `at`.
    pub fn touch_mac(&mut self, id: &str, at: i64) -> Result<(), DatabaseFault> {
        self.conn
            .execute("UPDATE pocket_macs SET last_heard_at = ?2 WHERE mac_id = ?1", (id, at))
            .map_err(fault("noting another Mac answered"))?;
        Ok(())
    }

    /// The link dropped. `since` is kept from the first drop, so a Mac asleep for hours says so.
    pub fn mac_asleep(&mut self, id: &str, since: i64, why: &str) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "UPDATE pocket_macs SET asleep_since = COALESCE(asleep_since, ?2), why = ?3 WHERE mac_id = ?1",
                (id, since, why),
            )
            .map_err(fault("noting another Mac is not answering"))?;
        Ok(())
    }

    /// Forget another Mac. `false` where there was none.
    pub fn remove_mac(&mut self, id: &str) -> Result<bool, DatabaseFault> {
        let gone = self
            .conn
            .execute("DELETE FROM pocket_macs WHERE mac_id = ?1", (id,))
            .map_err(fault("forgetting another Mac"))?;
        Ok(gone == 1)
    }
}
