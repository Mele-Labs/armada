//! Kit over the Mac link, worked out from what `get_kit_link` answered: which of Kit's kinds and items cross, what
//! each switch sends, how each row
//! stands against the linked Macs, what the bar across Kit says, and a clash's two sides as tinted lines. Nothing
//! here touches the screen or the wire. `docs/concepts/kit.md`, *Over the Mac link*.

use ipc::{HomeItem, HomeKind, KitAgainst, KitLink, KitLinkRow, KitMark};

use super::model::{Kind, Mark, Row};
use crate::look::{token, Token};

/// The kind the link calls one of Kit's. Output styles do not cross.
pub fn home_kind(kind: Kind) -> Option<HomeKind> {
    match kind {
        Kind::Skills => Some(HomeKind::Skills),
        Kind::Plugins => Some(HomeKind::Plugins),
        Kind::SubAgents => Some(HomeKind::SubAgents),
        Kind::Commands => Some(HomeKind::Commands),
        Kind::Instructions => Some(HomeKind::GlobalInstructions),
        Kind::Servers => Some(HomeKind::McpServers),
        Kind::Allowed => Some(HomeKind::Allowlist),
        Kind::OutputStyles => None,
    }
}

/// The name the link gives a row: a plugin by its name and its marketplace, as the harness names it.
pub fn link_name(row: &Row) -> String {
    match (row.kind, &row.marketplace) {
        (Kind::Plugins, Some(marketplace)) => format!("{}@{marketplace}", row.name),
        _ => row.name.clone(),
    }
}

/// The item an act on this row names.
pub fn item_of(row: &Row) -> Option<HomeItem> {
    Some(HomeItem { kind: home_kind(row.kind)?, name: link_name(row), repository: None })
}

/// Whether Kit's switch for `kind` is on here. None where the kind does not cross.
pub fn kind_on(link: &KitLink, kind: Kind) -> Option<bool> {
    let home = home_kind(kind)?;
    Some(link.kinds.iter().any(|one| one.kind == home && one.on))
}

/// Whether this row is switched on by itself, while its kind is off.
pub fn item_on(link: &KitLink, row: &Row) -> bool {
    item_of(row).is_some_and(|item| link.items.contains(&item))
}

/// How a kind's switch in the rail stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindCross {
    /// Every item of the kind crosses, the ones added later too.
    On,
    /// The kind is off and some of its items cross by themselves.
    Partly,
    Off,
}

/// None where the kind does not cross.
pub fn kind_cross(link: &KitLink, kind: Kind) -> Option<KindCross> {
    let home = home_kind(kind)?;
    Some(if kind_on(link, kind)? {
        KindCross::On
    } else if link.items.iter().any(|item| item.kind == home) {
        KindCross::Partly
    } else {
        KindCross::Off
    })
}

/// Whether a row crosses: its kind is on, or it is on by itself.
pub fn row_crosses(link: &KitLink, row: &Row) -> bool {
    kind_on(link, row.kind) == Some(true) || item_on(link, row)
}

/// Whether a row carries its own switch over the link: one of his, of a kind that crosses.
pub fn row_switchable(row: &Row) -> bool {
    his(row) && item_of(row).is_some()
}

/// Fleet's reason a row stays on this Mac, where it refuses it.
pub fn refusal<'a>(link: &'a KitLink, row: &Row) -> Option<&'a str> {
    link_row(link, row).and_then(|one| one.refused.as_deref())
}

/// One call over the link: `set_kit_link_kind` or `set_kit_link_item`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cross {
    Kind(HomeKind, bool),
    Item(HomeItem, bool),
}

/// What the rail's switch sends. From some or off, the kind on, which takes every item. From on, each item on by
/// itself off and then the kind, so a refusal part way leaves the kind as it stood.
pub fn rail_press(link: &KitLink, kind: Kind) -> Vec<Cross> {
    let Some(home) = home_kind(kind) else { return Vec::new() };
    match kind_cross(link, kind) {
        None => Vec::new(),
        Some(KindCross::On) => link.items.iter().filter(|item| item.kind == home).map(|item| Cross::Item(item.clone(), false)).chain([Cross::Kind(home, false)]).collect(),
        Some(_) => vec![Cross::Kind(home, true)],
    }
}

/// What a row's switch sends, `rows` being its kind's rows in Kit's list. With the kind off, the item by itself.
/// With the kind on, every other row Fleet does not refuse goes on by itself first, then the kind goes off, then
/// this item where it was on by itself: a refusal part way never leaves the rest uncrossed.
pub fn row_press(link: &KitLink, row: &Row, rows: &[Row]) -> Vec<Cross> {
    let Some(item) = item_of(row).filter(|_| row_switchable(row)) else { return Vec::new() };
    if refusal(link, row).is_some() {
        return Vec::new();
    }
    if kind_on(link, row.kind) != Some(true) {
        return vec![Cross::Item(item, !item_on(link, row))];
    }
    let mut sent: Vec<Cross> = rows
        .iter()
        .filter(|one| one.id != row.id && one.kind == row.kind && row_switchable(one) && refusal(link, one).is_none() && !item_on(link, one))
        .filter_map(item_of)
        .filter(|one| *one != item)
        .map(|one| Cross::Item(one, true))
        .collect();
    sent.push(Cross::Kind(item.kind, false));
    if link.items.contains(&item) {
        sent.push(Cross::Item(item, false));
    }
    sent
}

/// A row the person keeps himself: never a piece a plugin brought, a repository's, or Armada's own.
pub fn his(row: &Row) -> bool {
    row.comes_with.is_none() && !matches!(row.mark, Mark::Plugin(_) | Mark::Repository | Mark::Folder(_) | Mark::Armada)
}

/// The link's row for one of Kit's rows, where the row crosses.
pub fn link_row<'a>(link: &'a KitLink, row: &Row) -> Option<&'a KitLinkRow> {
    if !his(row) {
        return None;
    }
    let item = item_of(row)?;
    link.rows.iter().find(|one| one.kind == item.kind && one.name == item.name && one.repository.is_none())
}

/// How one row stands, as its mark says it. Each names the linked Macs it stands so against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Same(String),
    /// Changed on one Mac since they last agreed: it crosses by itself.
    Differs(String),
    /// Both changed it since they last agreed: each keeps its own until he picks.
    Clash(String),
    NewHere(String),
    /// Only on that Mac. Its words name the Mac.
    OnlyThere(String),
    /// This Mac's version stays here, in Fleet's words.
    Refused(String),
}

impl Standing {
    /// The registry glyph: the shared home's `check` and `split`, `laptop` for this Mac and `monitor` for another.
    pub fn glyph(&self) -> &'static str {
        match self {
            Standing::Same(_) => "check",
            Standing::Differs(_) => "git-compare",
            Standing::Clash(_) => "split",
            Standing::NewHere(_) => "laptop",
            Standing::OnlyThere(_) => "monitor",
            Standing::Refused(_) => "x",
        }
    }

    pub fn hue(&self) -> Token {
        match self {
            Standing::Same(_) => token::FG_MUTED,
            Standing::Differs(_) => token::STATUS_RUNNING,
            Standing::Clash(_) => token::STATUS_AWAITING_REVIEW,
            Standing::NewHere(_) => token::ACCENT,
            Standing::OnlyThere(_) => token::STATUS_HANDED_IN,
            Standing::Refused(_) => token::STATUS_COMPLETED_FAILED,
        }
    }

    /// The mark's own words, in its column.
    pub fn word(&self) -> String {
        match self {
            Standing::Same(_) => "Same".into(),
            Standing::Differs(_) => "Differs".into(),
            Standing::Clash(_) => "Clash".into(),
            Standing::NewHere(_) => "New here".into(),
            Standing::OnlyThere(macs) => format!("Only on {macs}"),
            Standing::Refused(_) => "Refused".into(),
        }
    }

    /// What its tooltip says.
    pub fn said(&self) -> String {
        match self {
            Standing::Same(macs) => format!("Same on {macs}"),
            Standing::Differs(macs) => format!("Changed on one Mac since this Mac and {macs} last agreed. It crosses by itself."),
            Standing::Clash(macs) => format!("This Mac and {macs} both changed it. Pick one in its detail."),
            Standing::NewHere(macs) => format!("Only on this Mac. {macs} takes it at the next exchange."),
            Standing::OnlyThere(macs) => format!("On {macs} and not on this Mac"),
            Standing::Refused(why) => why.clone(),
        }
    }
}

/// The loudest mark first: a clash, then a difference, then what is on one side alone.
fn loudness(mark: KitMark) -> u8 {
    match mark {
        KitMark::Clash => 5,
        KitMark::Differs => 4,
        KitMark::OnlyThere => 3,
        KitMark::NewHere => 2,
        KitMark::Same => 1,
    }
}

/// A linked Mac's name, by its id.
pub fn mac_name(link: &KitLink, mac: &str) -> String {
    link.macs.iter().find(|one| one.mac == mac).map_or_else(|| mac.to_string(), |one| one.name.clone())
}

/// Names as a person lists them: `A`, `A and B`, `A, B and C`.
fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// How a link row stands: Fleet's refusal before anything, then its loudest mark against any linked Mac, naming
/// every Mac it stands so against. None where no linked Mac crosses it.
pub fn standing(link: &KitLink, row: &KitLinkRow) -> Option<Standing> {
    if let Some(why) = &row.refused {
        return Some(Standing::Refused(why.clone()));
    }
    let loudest = row.against.iter().map(|one| one.mark).max_by_key(|mark| loudness(*mark))?;
    let names: Vec<String> = row.against.iter().filter(|one| one.mark == loudest).map(|one| mac_name(link, &one.mac)).collect();
    let macs = listed(&names);
    Some(match loudest {
        KitMark::Same => Standing::Same(macs),
        KitMark::Differs => Standing::Differs(macs),
        KitMark::Clash => Standing::Clash(macs),
        KitMark::NewHere => Standing::NewHere(macs),
        KitMark::OnlyThere => Standing::OnlyThere(macs),
    })
}

/// The rows of `kind` that are on a linked Mac and not here, as Kit's list holds them: in his own group, with no
/// file or act of this Mac's.
pub fn elsewhere(link: &KitLink, kind: Kind, rows: &[Row]) -> Vec<Row> {
    let Some(home) = home_kind(kind) else { return Vec::new() };
    let here: Vec<String> = rows.iter().filter(|row| his(row)).map(link_name).collect();
    link.rows
        .iter()
        .filter(|one| one.kind == home && one.repository.is_none() && !here.contains(&one.name))
        .filter(|one| one.against.iter().any(|against| against.mark == KitMark::OnlyThere))
        .map(|one| {
            let (name, marketplace) = match (kind, one.name.split_once('@')) {
                (Kind::Plugins, Some((plain, marketplace))) => (plain.to_string(), Some(marketplace.to_string())),
                _ => (one.name.clone(), None),
            };
            let there: Vec<String> = one.against.iter().filter(|against| against.mark == KitMark::OnlyThere).map(|against| mac_name(link, &against.mac)).collect();
            Row {
                id: format!("{}:link:{}", kind.id(), one.name),
                kind,
                name,
                says: one.says.clone(),
                mark: Mark::Yours,
                path: None,
                piece: None,
                comes_with: None,
                plugin_id: None,
                jobs: None,
                server: None,
                in_force: None,
                command: None,
                marketplace,
                file: None,
                elsewhere: Some(listed(&there)),
            }
        })
        .collect()
}

/// The linked Macs this row clashes with, each with both sides.
pub fn clashes(row: &KitLinkRow) -> Vec<&KitAgainst> {
    row.against.iter().filter(|one| one.mark == KitMark::Clash && one.here.is_some() && one.there.is_some()).collect()
}

/// Where his Kit stands against his linked Macs, for the bar across Kit and the Account page's Kit row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncStatus {
    /// The linked Macs, by name.
    pub macs: Vec<String>,
    /// Nothing differs and nothing clashes. What is on one Mac alone, or refused, does not count against it.
    pub in_step: bool,
}

/// None where no Mac is linked.
pub fn sync_status(link: &KitLink) -> Option<SyncStatus> {
    if link.macs.is_empty() {
        return None;
    }
    let apart = link.rows.iter().flat_map(|row| row.against.iter()).any(|one| matches!(one.mark, KitMark::Differs | KitMark::Clash));
    Some(SyncStatus { macs: link.macs.iter().map(|mac| mac.name.clone()).collect(), in_step: !apart })
}

/// What differs, for the bar's chip: whether any of it clashes, and the items, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Apart {
    pub clash: bool,
    pub names: Vec<String>,
}

/// What the bar across Kit says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bar {
    Unlinked,
    Linked {
        /// The Macs it crosses to, as a person lists them.
        to: String,
        /// Each linked Mac and when its Kit last crossed here.
        crossed: Vec<(String, Option<ipc::Instant>)>,
        /// Set where something differs. A clash outranks a difference.
        apart: Option<Apart>,
    },
}

pub fn bar(link: &KitLink) -> Bar {
    let Some(status) = sync_status(link) else { return Bar::Unlinked };
    let apart = (!status.in_step).then(|| {
        let clash = link.rows.iter().any(|row| row.against.iter().any(|one| one.mark == KitMark::Clash));
        let wanted = if clash { KitMark::Clash } else { KitMark::Differs };
        let names = link.rows.iter().filter(|row| row.against.iter().any(|one| one.mark == wanted)).map(|row| row.name.clone()).collect();
        Apart { clash, names }
    });
    Bar::Linked { to: listed(&status.macs), crossed: link.macs.iter().map(|mac| (mac.name.clone(), mac.crossed_at.clone())).collect(), apart }
}

/// The bar's chip words and tooltip for what differs.
pub fn apart_said(apart: &Apart) -> (&'static str, String) {
    let names = listed(&apart.names);
    if apart.clash {
        ("Clash", format!("Both Macs changed {names}. Pick one in its detail."))
    } else {
        ("Differs", format!("{names} changed on one side. It crosses by itself."))
    }
}

/// A line of one side of a clash, against the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    /// On both sides.
    Both,
    /// On this side only.
    Ours,
    /// On the other side only, drawn where it would sit.
    Theirs,
}

/// One side read against the other, line by line: what both hold, what this side holds alone, and where the
/// other's own lines sit. A longest common run of lines; Kit's files are short.
pub fn compared(ours: &str, theirs: &str) -> Vec<(Tint, String)> {
    let a: Vec<&str> = ours.lines().collect();
    let b: Vec<&str> = theirs.lines().collect();
    let (n, m) = (a.len(), b.len());
    // Past a million pairs the table is not worth its memory: the side is drawn as it is, untinted.
    if n.saturating_mul(m) > 1_000_000 {
        return a.iter().map(|line| (Tint::Both, line.to_string())).collect();
    }
    let mut run = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            run[i][j] = if a[i] == b[j] { run[i + 1][j + 1] + 1 } else { run[i + 1][j].max(run[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(n.max(m));
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((Tint::Both, a[i].to_string()));
            i += 1;
            j += 1;
        } else if run[i + 1][j] >= run[i][j + 1] {
            out.push((Tint::Ours, a[i].to_string()));
            i += 1;
        } else {
            out.push((Tint::Theirs, b[j].to_string()));
            j += 1;
        }
    }
    out.extend(a[i..].iter().map(|line| (Tint::Ours, line.to_string())));
    out.extend(b[j..].iter().map(|line| (Tint::Theirs, line.to_string())));
    out
}
