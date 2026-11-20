//! Kit over the Mac link on the wire: each kind's switch on this Mac, each row marked against every linked Mac, the
//! acts that switch a kind or an item and settle a clash, and what one Mac offers another. `docs/concepts/kit.md`,
//! *Over the Mac link*.
//!
//! **A clash carries both sides**, each with its Mac's name and when that Mac first read its version, so Kit can
//! show them side by side without a second read. Nothing is merged.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{HomeItem, HomeKind, HomeKindSwitch, HomeUnreadable, Instant, ModKind};

/// How a row stands against one linked Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitMark {
    Same,
    /// On both, changed on one side only since they last agreed: it crosses on the next exchange, or could not.
    Differs,
    /// Only on this Mac.
    NewHere,
    /// Only on the other Mac.
    OnlyThere,
    /// Both Macs changed it since they last agreed. Each keeps its own until he picks.
    Clash,
}

/// One Mac's version of an item, as a clash or a difference shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitSide {
    /// The Mac's name, as he named it.
    pub mac_name: String,
    /// Its one file, or each file under its path.
    pub text: String,
    /// When that Mac first read this version. Absent where it was never read there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<Instant>,
}

/// One row against one linked Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitAgainst {
    /// The linked Mac's id.
    pub mac: String,
    pub mark: KitMark,
    /// This Mac's version, on a row that differs or clashes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub here: Option<KitSide>,
    /// The other Mac's version, on a row that differs or clashes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub there: Option<KitSide>,
}

/// One Kit item that crosses, or would.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitLinkRow {
    pub kind: HomeKind,
    pub name: String,
    /// A memory note's repository, by its folder's name, which names the row in an act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// Where it comes from: a memory note's repository, an allowlist line's origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// On an MCP server alone: `false` where this Mac has no server of that name, so its key is not here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_here: Option<bool>,
    /// On a mod alone: the kind its `mod.toml` names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_kind: Option<ModKind>,
    /// Why this Mac's version stays here: it carries a credential, or a mod that is not valid. Names a host,
    /// never the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
    /// Against each linked Mac that crosses this row's kind or this item. Empty where none does.
    #[serde(default)]
    pub against: Vec<KitAgainst>,
}

/// A linked Mac Kit crosses to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitLinkedMac {
    pub mac: String,
    pub name: String,
    /// When its Kit last crossed to this Mac. Absent where it never has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossed_at: Option<Instant>,
    /// The kinds it has switched on, as it last said.
    #[serde(default)]
    pub kinds: Vec<HomeKind>,
    /// The items it switched on by themselves.
    #[serde(default)]
    pub items: Vec<HomeItem>,
}

/// `get_kit_link`, and what every act on it answers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitLink {
    /// Every kind in order, with this Mac's switch: off until he switches it on.
    pub kinds: Vec<HomeKindSwitch>,
    /// Items switched on by themselves while their kind is off.
    #[serde(default)]
    pub items: Vec<HomeItem>,
    /// Every Mac this one is linked to, asleep or awake.
    #[serde(default)]
    pub macs: Vec<KitLinkedMac>,
    /// Every row of each kind switched on, and each item on by itself, kind by kind in [`HomeKind::ALL`]'s order.
    #[serde(default)]
    pub rows: Vec<KitLinkRow>,
    #[serde(default)]
    pub unreadable: Vec<HomeUnreadable>,
}

/// `set_kit_link_kind`: one kind switched on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetKitLinkKind {
    pub kind: HomeKind,
    pub on: bool,
}

/// `set_kit_link_item`: one item switched by itself on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetKitLinkItem {
    #[serde(flatten)]
    pub item: HomeItem,
    pub on: bool,
}

/// Which side of a clash is kept, on both Macs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitKeep {
    /// This Mac's.
    Here,
    /// The other Mac's.
    There,
}

/// `settle_kit_clash`: keep one side of a clash with Mac `mac`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleKitClash {
    pub mac: String,
    #[serde(flatten)]
    pub item: HomeItem,
    pub keep: KitKeep,
}

/// One item as one Mac offers it to another: its files whole, and the hash both compare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitPiece {
    pub kind: HomeKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Each file's text by its path relative to where the kind is kept. Empty for an allowlist line.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The hash of its canonical text, which a last-agreed marker records.
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<Instant>,
    /// The offering Mac's last-agreed marker for it with the Mac that asked, so the asker can tell an edit made
    /// there after taking the asker's from an edit made on both. Left out where they never agreed on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreed: Option<String>,
}

/// `share_kit`: what this Mac offers his other Macs. Only the kinds it has on and the items on by themselves, and
/// never a piece that carries a credential.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitOffer {
    /// The offering Mac's name, as a clash names its side.
    #[serde(default)]
    pub name: String,
    pub kinds: Vec<HomeKind>,
    #[serde(default)]
    pub items: Vec<HomeItem>,
    #[serde(default)]
    pub pieces: Vec<KitPiece>,
}

/// `hold_other_mac_kit`: what Mac `mac` offers, handed in by this Mac's Phone Gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherMacKit {
    pub mac: String,
    pub kit: KitOffer,
}

/// `kit_settle_from_mac`: a clash the sending Mac settled. Both Macs record `agreed`, the hash of the side not kept,
/// so the kept side reads as the only change and crosses to the Mac that did not have it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitSettled {
    #[serde(flatten)]
    pub item: HomeItem,
    pub agreed: String,
}

/// `kit.changed`: this Mac's Kit or how it stands against a linked Mac changed. Bridge reads `get_kit_link` again,
/// and a linked Mac reads `share_kit` again. Carries nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitChanged {}
