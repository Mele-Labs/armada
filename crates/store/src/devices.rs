//! The phones paired with the Phone Gateway. Times are Unix seconds, read by the
//! caller from its own clock, so a test sets them.

use crate::error::{fault, DatabaseFault};
use crate::open::Store;

/// A paired phone. `public_key` is the SPKI DER it exported at pairing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub public_key: Vec<u8>,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
}

/// A phone's Web Push subscription, as the browser's `PushSubscription` gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushSubscription {
    pub device_id: String,
    pub endpoint: String,
    /// The phone's P-256 key, base64url.
    pub p256dh: String,
    /// The phone's 16-byte authentication secret, base64url.
    pub auth: String,
}

const COLUMNS: &str = "device_id, name, public_key, created_at, last_seen_at";

fn device(row: &rusqlite::Row<'_>) -> rusqlite::Result<Device> {
    Ok(Device {
        id: row.get(0)?,
        name: row.get(1)?,
        public_key: row.get(2)?,
        created_at: row.get(3)?,
        last_seen_at: row.get(4)?,
    })
}

impl Store {
    pub fn add_device(&mut self, device: &Device) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "INSERT INTO pocket_devices (device_id, name, public_key, created_at, last_seen_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    &device.id,
                    &device.name,
                    &device.public_key,
                    device.created_at,
                    device.last_seen_at,
                ),
            )
            .map_err(fault("pairing a phone"))?;
        Ok(())
    }

    pub fn paired_device(&self, id: &str) -> Result<Option<Device>, DatabaseFault> {
        let mut asking = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM pocket_devices WHERE device_id = ?1"))
            .map_err(fault("reading a paired phone"))?;
        let mut rows = asking
            .query_map((id,), device)
            .map_err(fault("reading a paired phone"))?;
        rows.next()
            .transpose()
            .map_err(fault("reading a paired phone"))
    }

    /// Every paired phone, oldest first.
    pub fn paired_devices(&self) -> Result<Vec<Device>, DatabaseFault> {
        let doing = "reading the paired phones";
        let mut asking = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM pocket_devices ORDER BY created_at, device_id"
            ))
            .map_err(fault(doing))?;
        let rows = asking.query_map([], device).map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    pub fn touch_device(&mut self, id: &str, at: i64) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "UPDATE pocket_devices SET last_seen_at = ?2 WHERE device_id = ?1",
                (id, at),
            )
            .map_err(fault("noting when a phone was last seen"))?;
        Ok(())
    }

    /// Unpair a phone and drop its push subscriptions. `false` where there was none.
    pub fn remove_device(&mut self, id: &str) -> Result<bool, DatabaseFault> {
        let doing = "unpairing a phone";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        tx.execute("DELETE FROM pocket_push_subscriptions WHERE device_id = ?1", (id,))
            .map_err(fault(doing))?;
        let gone = tx
            .execute("DELETE FROM pocket_devices WHERE device_id = ?1", (id,))
            .map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))?;
        Ok(gone == 1)
    }

    /// Keep this subscription for the device in place of any it had before.
    pub fn set_push_subscription(&mut self, sub: &PushSubscription) -> Result<(), DatabaseFault> {
        let doing = "saving a phone's push subscription";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        tx.execute("DELETE FROM pocket_push_subscriptions WHERE device_id = ?1", (&sub.device_id,))
            .map_err(fault(doing))?;
        tx.execute(
            "INSERT INTO pocket_push_subscriptions (device_id, endpoint, p256dh, auth) VALUES (?1, ?2, ?3, ?4)",
            (&sub.device_id, &sub.endpoint, &sub.p256dh, &sub.auth),
        )
        .map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))
    }

    pub fn push_subscriptions(&self) -> Result<Vec<PushSubscription>, DatabaseFault> {
        let doing = "reading the push subscriptions";
        let mut asking = self
            .conn
            .prepare("SELECT device_id, endpoint, p256dh, auth FROM pocket_push_subscriptions ORDER BY device_id")
            .map_err(fault(doing))?;
        let rows = asking
            .query_map([], |row| {
                Ok(PushSubscription {
                    device_id: row.get(0)?,
                    endpoint: row.get(1)?,
                    p256dh: row.get(2)?,
                    auth: row.get(3)?,
                })
            })
            .map_err(fault(doing))?;
        rows.collect::<Result<_, _>>().map_err(fault(doing))
    }

    /// Drop a subscription the push service said is gone.
    pub fn remove_push_subscription(&mut self, device_id: &str, endpoint: &str) -> Result<(), DatabaseFault> {
        self.conn
            .execute(
                "DELETE FROM pocket_push_subscriptions WHERE device_id = ?1 AND endpoint = ?2",
                (device_id, endpoint),
            )
            .map_err(fault("dropping a push subscription"))?;
        Ok(())
    }
}
