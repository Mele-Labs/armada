//! What a node and a rule wear: their marks, hues and frames, in Classic's order.

use gpui::{Hsla, Rgba};
use ipc::{LaneAutonomy, LaneEdge, LaneNode, LaneNodeKind, LaneOutcome, LanePending};

use super::layout::meta;
use super::Wears;
use crate::look::{token, Look};

/// What a node or a chip is drawn as, so a change to it can be told (`markOfNode`, `markOfRule`).
pub(super) fn mark_of_node(node: &LaneNode) -> String {
    format!("{}|{}|{}|{:?}|{}|{:?}|{:?}|{:?}|{:?}", node.lit, node.here, node.passed, node.waiting, node.skipped, node.ticket.as_ref().map(|one| (&one.label, &one.state_id, &one.state)), node.changed, node.differs, node.clash)
}
pub(super) fn mark_of_rule(edge: &LaneEdge) -> String {
    format!("{:?}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}", edge.status, meta(edge), edge.proposed, edge.pending, edge.autonomy, edge.did, edge.changed, edge.differs, edge.clash)
}

/// What a piece's clock waits on, which is its tooltip (`PENDING_SAID`).
pub(super) fn pending_said(pending: LanePending) -> &'static str {
    match pending {
        LanePending::Removal => "Removed from the repository when its pull request merges",
        LanePending::Keep => "Kept for every Job once its draft is approved",
    }
}

/// The places a node or rule is set otherwise in (`differsSaid`).
pub(super) fn differs_said(names: &[String]) -> String {
    format!("Differs in {}", names.join(", "))
}

/// A node's mark and what its tooltip says of it (`markOf`).
pub(super) fn mark_of(node: &LaneNode) -> Option<(&'static str, &'static str)> {
    if node.skipped {
        Some(("ban", "Skipped"))
    } else if node.ended {
        Some(("ban", "Ended"))
    } else if !node.waiting.is_empty() {
        Some(("clock", "Waiting"))
    } else if node.lit {
        Some(("circle-dot", "Working"))
    } else if node.here {
        Some(("map-pin", "Now"))
    } else if node.passed {
        Some(("check", "Went through"))
    } else {
        None
    }
}

/// The glyph of a ticket's state on a Job's card (`TICKET_STATE`); a state none names reads as unknown.
pub(super) fn ticket_state(id: &str) -> &'static str {
    match id {
        "todo" => "circle-dashed",
        "in_progress" => "circle-dot",
        "in_review" => "eye",
        "done" | "closed" => "circle-check",
        _ => "circle-question-mark",
    }
}

/// The mark on a rule's chip (`chipMark`): what it waits on, a rule a draft would add, or how it ended on the Job.
pub(super) fn chip_mark(edge: &LaneEdge) -> Option<(&'static str, &'static str)> {
    if let Some(pending) = edge.pending {
        return Some(("clock", pending_said(pending)));
    }
    if edge.proposed {
        return Some(("list-plus", "Proposed"));
    }
    match edge.status {
        LaneOutcome::Acted => Some(("check", "Acted")),
        LaneOutcome::Asked => Some(("message-circle-question-mark", "Asked")),
        LaneOutcome::Waiting => Some(("clock", "Waiting")),
        LaneOutcome::Skipped => Some(("ban", "Skipped")),
    }
}

/// `color-mix(in srgb, a p, b)`.
pub(crate) fn mix(a: Hsla, b: Hsla, p: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    Rgba { r: a.r * p + b.r * (1. - p), g: a.g * p + b.g * (1. - p), b: a.b * p + b.b * (1. - p), a: a.a * p + b.a * (1. - p) }.into()
}

/// The picked frame and ground (`--select-edge`, `--select-ground`, `takes.css`).
pub(super) fn select_edge(look: &Look) -> Hsla {
    let fg = look.color(token::FG_DEFAULT);
    fg.opacity(fg.a * 0.7)
}
pub(super) fn select_ground(look: &Look) -> Hsla {
    let tint = look.percent(token::SELECT_TINT);
    mix(look.color(token::ACCENT), look.color(token::BG_BASE), if tint > 0. { tint } else { 0.12 })
}

/// The tint of what a node is (`--kind`), which carries no status.
pub(super) fn kind_tint(look: &Look, kind: LaneNodeKind) -> Hsla {
    look.color(match kind {
        LaneNodeKind::Watch => token::KIND_GATE,
        LaneNodeKind::Tracker => token::KIND_SETUP,
        LaneNodeKind::Forge => token::KIND_DELIVERY,
        _ => token::KIND_STEP,
    })
}

pub(super) fn wears(look: &Look, node: &LaneNode, picked: bool) -> Wears {
    let kind = kind_tint(look, node.kind);
    let mut w = Wears {
        hue: look.color(token::FG_SUBTLE),
        frame: look.color(token::BORDER_STRONG),
        ground: mix(kind, look.color(token::BG_RAISED), 0.08),
        ink: look.color(token::FG_DEFAULT),
        heavy: false,
        inset: None,
        dashed: false,
    };
    if node.here {
        w.hue = look.color(token::FG_DEFAULT);
        w.frame = look.color(token::FG_MUTED);
        w.ground = look.color(token::BG_HOVER);
        w.inset = Some(look.color(token::FG_MUTED));
        w.heavy = true;
    }
    if node.passed {
        w.hue = look.color(token::STEP_ADVANCED);
        w.frame = mix(look.color(token::STEP_ADVANCED), look.color(token::BORDER_STRONG), 0.45);
        w.ink = look.color(token::FG_MUTED);
    }
    if node.lit {
        w.hue = look.color(token::STEP_RUNNING);
        w.frame = look.color(token::STEP_RUNNING);
        w.ground = mix(look.color(token::STEP_RUNNING), look.color(token::BG_RAISED), 0.1);
        w.heavy = true;
    }
    if !node.waiting.is_empty() {
        w.hue = look.color(token::STATUS_AWAITING_REVIEW);
        w.frame = look.color(token::STATUS_AWAITING_REVIEW);
        w.ground = mix(look.color(token::STATUS_AWAITING_REVIEW), look.color(token::BG_RAISED), 0.1);
    }
    if node.skipped || node.ended {
        w.hue = look.color(token::FG_SUBTLE);
        w.ink = look.color(token::FG_SUBTLE);
    }
    if node.changed.is_some() {
        w.dashed = true;
        w.frame = look.color(token::STATUS_AWAITING_REVIEW);
    }
    if picked {
        w.frame = select_edge(look);
        w.ground = select_ground(look);
        w.inset = Some(select_edge(look));
        w.heavy = true;
    }
    w
}

/// A rule's hue (`.armada-lane-edge[data-tone]`), and whether its line and diamond are dashed.
pub(super) fn rule_hue(look: &Look, edge: &LaneEdge, picked: bool) -> (Hsla, bool) {
    let dashed = edge.proposed || matches!(edge.status, LaneOutcome::Waiting | LaneOutcome::Skipped);
    let hue = if picked {
        look.color(token::FG_DEFAULT)
    } else if edge.autonomy == LaneAutonomy::Off {
        look.color(token::BORDER_STRONG)
    } else if edge.proposed {
        look.color(token::STEP_ADVANCED)
    } else {
        match edge.status {
            LaneOutcome::Acted => look.color(token::STEP_ADVANCED),
            LaneOutcome::Asked => look.color(token::STATUS_AWAITING_REVIEW),
            _ => look.color(token::FG_MUTED),
        }
    };
    (hue, dashed)
}
