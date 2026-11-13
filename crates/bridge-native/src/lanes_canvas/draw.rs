//! The canvas's elements: a lane's head, a node and a rule's chip.

use gpui::{canvas, div, point, prelude::*, px, AnyElement, Context, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString};
use ipc::{JobLanesView, LaneAutonomy, LaneEdge, LaneKind, LaneNode, LaneNodeKind, LaneView};

use super::layout::meta;
use super::paint::paint_polygon;
use super::wears::{chip_mark, differs_said, kind_tint, mark_of, mix, pending_said, rule_hue, select_edge, select_ground, ticket_state, wears};
use super::{lanes_press, LanesHost, Pick, Rect, CHIP_H, HEAD};
use crate::job_view::rows::{glyph, mono, tip, Size};
use crate::labels::labelled;
use crate::look::{token, Look};

/// The focus ring round the node the keys are on.
pub(super) fn ring(look: &Look, rect: Rect, at: (f32, f32), z: f32, round: bool) -> gpui::Div {
    let gap = look.length(token::BORDER_WIDTH) * 2.;
    div()
        .absolute()
        .left(px(at.0) - gap)
        .top(px(at.1) - gap)
        .w(px(rect.w * z) + gap * 2.)
        .h(px(rect.h * z) + gap * 2.)
        .border(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::ACCENT))
        .when(round, |ring| ring.rounded_full())
        .when(!round, |ring| ring.rounded(look.length(token::RADIUS_MD) * z + gap))
}

/// A glyph `size` px across at the canvas's zoom.
pub(super) fn zoomed_glyph(icon: &'static str, z: f32, ink: Hsla, size: f32) -> gpui::Svg {
    gpui::svg().path(crate::assets::icon_path(icon)).flex_none().size(px(size * z)).text_color(ink)
}

/// A lane's head, over the left of the frame at the lane's own height: its title and where its states
/// are read from (`Heads`). A ticket's head opens a menu with Teach; the head of a Job this one started,
/// where that Job has lanes, opens them on a double press or Enter.
#[allow(clippy::too_many_arguments)]
pub(super) fn lane_head<H: LanesHost>(host: &H, id: &SharedString, lanes: &JobLanesView, lane: &LaneView, top: f32, height: f32, look: &Look, cx: &Context<H>) -> AnyElement {
    let workflow = lane.kind == LaneKind::Workflow;
    let source = if lane.ended { "Ended".to_string() } else { lane.what.clone().unwrap_or_else(|| lane.source.clone()) };
    let opens = !lane.ticket && host.head_opens(lanes, lane, cx);
    let lines: Vec<&str> = lane.note.iter().map(String::as_str).chain(opens.then_some(super::OPENS_SAID)).collect();
    let said = (!lines.is_empty()).then(|| format!("{}\n{}", lane.title, lines.join("\n")));
    let changed = lane.changed.as_ref().map(|changed| {
        div()
            .id(SharedString::from(format!("lane-head-changed:{}", lane.id)))
            .absolute()
            .top(look.length(token::SPACE_1))
            // Beside a wait, a change on this Job stands to its left.
            .right(if lane.pending.is_some() { look.length(token::SPACE_1) + look.length(token::SPACE_4) } else { look.length(token::SPACE_1) })
            .tooltip(tip(format!("Changed on this Job\n{changed}")))
            .child(glyph(look, "git-compare", Size::Small, look.color(token::STATUS_AWAITING_REVIEW)))
    });
    // What the lane's lifecycle waits on before it is kept or goes.
    let pending = lane.pending.map(|pending| {
        let said = pending_said(pending);
        labelled(
            div()
                .id(SharedString::from(format!("lane-head-pending:{}", lane.id)))
                .absolute()
                .top(look.length(token::SPACE_1))
                .right(look.length(token::SPACE_1))
                // On two lines, so it fits to the clock's right and leaves the head it describes in view.
                .tooltip(tip(said.replacen(" when ", "\nwhen ", 1).replacen(" once ", "\nonce ", 1)))
                .child(glyph(look, "clock", Size::Small, look.color(token::FG_SUBTLE))),
            Role::Image,
            said,
        )
    });
    let words = div()
        .id(SharedString::from(format!("lane-head-words:{}", lane.id)))
        .size_full()
        .flex()
        .flex_col()
        .justify_center()
        .gap(look.length(token::SPACE_1))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .when_some(said, |head, said| head.tooltip(tip(said)))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(if lane.ended { token::FG_SUBTLE } else { token::FG_DEFAULT }))
                .child(SharedString::from(lane.title.clone())),
        )
        .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(source)));
    let words = host.head_words(id, lane, opens, words, look, cx);
    let head = div()
        .id(SharedString::from(format!("lane-head:{}", lane.id)))
        .absolute()
        .left_0()
        .top(px(top))
        .h(px(height))
        .w(px(HEAD))
        .border_r(look.length(token::BORDER_WIDTH))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(if workflow { token::BORDER_STRONG } else { token::BORDER_SUBTLE }))
        .bg(look.color(if workflow { token::BG_BASE } else { token::BG_RAISED }))
        .font_family(mono(look))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(words)
        .children(pending)
        .children(changed);
    labelled(head, Role::Group, lane.title.clone()).into_any_element()
}

/// One node: its shape, mark, name and what it waits for (`ItemNode`).
#[allow(clippy::too_many_arguments)]
pub(super) fn lane_node<H: LanesHost>(id: &SharedString, lane: &LaneView, node: &LaneNode, at: (f32, f32), rect: Rect, z: f32, mine: bool, picked: bool, look: &Look, cx: &Context<H>) -> AnyElement {
    let dot = matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End);
    let ring = mine.then(|| ring(look, rect, at, z, dot));
    let s = |token| look.length(token) * z;
    // Where the workflow starts and ends: a circle with nothing to pick (`dot`).
    if dot {
        let frame = if node.here {
            look.color(token::FG_DEFAULT)
        } else if node.passed {
            look.color(token::STEP_ADVANCED)
        } else {
            look.color(token::FG_SUBTLE)
        };
        let border = look.length(token::BORDER_WIDTH) * 2. * z;
        let circle = div()
            .id(SharedString::from(format!("lane-dot:{}", node.id)))
            .absolute()
            .left(px(at.0))
            .top(px(at.1))
            .size(px(rect.w * z))
            .rounded_full()
            .border(border)
            .border_color(frame)
            .bg(frame)
            .tooltip(tip(node.label.clone()))
            .when(node.kind == LaneNodeKind::End, |circle| {
                // A ring round a dot: the sunken ground inset inside the frame.
                circle.flex().items_center().justify_center().child(div().size(px(rect.w * z) - border * 2.).rounded_full().border(s(token::SPACE_1)).border_color(look.color(token::BG_SUNKEN)).bg(frame))
            });
        return div().children(ring).child(labelled(circle, Role::Image, node.label.clone())).into_any_element();
    }
    let w = wears(look, node, picked);
    let card = node.kind == LaneNodeKind::Job;
    let pill = matches!(node.kind, LaneNodeKind::Tracker | LaneNodeKind::Forge);
    let hexagon = node.kind == LaneNodeKind::Watch;
    let mark = mark_of(node);
    let name = format!(
        "{}: {}{}{}",
        if lane.kind == LaneKind::Workflow { "Step" } else { lane.title.as_str() },
        node.label,
        node.ticket.as_ref().map_or(String::new(), |ticket| format!(", ticket {}{}", ticket.label, if ticket.state.is_empty() { String::new() } else { format!(", {}", ticket.state) })),
        if node.ended { ", ended" } else { "" }
    );
    let mut lines = vec![node.label.clone()];
    if node.waiting.is_empty() {
        lines.extend(mark.map(|(_, said)| said.to_string()));
    } else {
        lines.extend(node.waiting.iter().cloned());
    }
    if let Some(ticket) = &node.ticket {
        lines.push(match &ticket.title {
            Some(title) => format!("Ticket {}: {title}", ticket.label),
            None => format!("Ticket {}", ticket.label),
        });
        if !ticket.state.is_empty() {
            lines.push(format!("Ticket state: {}", ticket.state));
        }
        lines.push(format!("Held by {}", lane.title));
    }
    if !node.differs.is_empty() {
        lines.push(differs_said(&node.differs));
    }
    lines.extend(node.clash.clone());
    let kind = kind_tint(look, node.kind);
    let mark_view = div()
        .flex_none()
        .w(s(token::SPACE_6))
        .flex()
        .items_center()
        .justify_center()
        .children(if card { Some(zoomed_glyph("drone", z, kind, 16.)) } else { mark.map(|(icon, _)| zoomed_glyph(icon, z, w.hue, 16.)) });
    let sub = (!node.waiting.is_empty()).then(|| {
        div()
            .min_w_0()
            .line_clamp(2)
            .text_size(s(token::TEXT_XS))
            .line_height(s(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
            .text_color(look.color(token::STATUS_AWAITING_REVIEW))
            .child(SharedString::from(node.waiting.join("\n")))
    });
    let ticket = node.ticket.as_ref().map(|ticket| {
        div()
            .flex()
            .items_center()
            .gap(s(token::SPACE_1))
            .min_w_0()
            .text_size(s(token::TEXT_XS))
            .line_height(s(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(zoomed_glyph("ticket", z, look.color(token::FG_MUTED), 14.))
            .child(div().min_w_0().truncate().child(SharedString::from(ticket.label.clone())))
            .child(zoomed_glyph(ticket_state(&ticket.state_id), z, look.color(token::FG_MUTED), 14.))
    });
    let words = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(div().min_w_0().when(node.skipped, |label| label.line_through()).child(SharedString::from(node.label.clone())))
        .children(sub)
        .children(ticket);
    let status = card.then(|| mark.map(|(icon, _)| div().flex_none().pl(s(token::SPACE_1)).child(zoomed_glyph(icon, z, w.hue, 14.)))).flatten();
    let inset = w.inset;
    let body = div()
        .id(SharedString::from(format!("lane-node:{}", node.id)))
        .absolute()
        .left(px(at.0))
        .top(px(at.1))
        .w(px(rect.w * z))
        .h(px(rect.h * z))
        .flex()
        .items_center()
        .pt(s(token::SPACE_1))
        .pb(s(token::SPACE_1))
        .pl(if pill { s(token::SPACE_2) } else if hexagon { s(token::SPACE_4) } else { s(token::SPACE_1) })
        .pr(if hexagon { s(token::SPACE_4) } else if pill { s(token::SPACE_3) } else { s(token::SPACE_2) })
        .text_size(s(token::TEXT_SM))
        .line_height(s(token::LEADING_SM))
        .text_color(w.ink)
        .when(w.heavy, |body| body.font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
        .when(!hexagon, |body| {
            body.bg(w.ground)
                .border(look.length(token::BORDER_WIDTH) * z)
                .border_color(w.frame)
                .when(w.dashed, |body| body.border_dashed())
                .rounded(if pill { s(token::SPACE_12) * 2. } else if card { s(token::RADIUS_SM) } else { s(token::RADIUS_MD) })
                .when(card, |body| body.border_l(look.length(token::BORDER_WIDTH) * 3. * z))
        })
        .when_some(inset.filter(|_| !hexagon), |body, inset| {
            body.shadow(vec![gpui::BoxShadow { color: inset, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH) * z, inset: true }])
        })
        .cursor_pointer()
        .tooltip(tip(lines.join("\n")))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click({
            let (owner, node_id) = (id.clone(), node.id.clone());
            cx.listener(move |this: &mut H, _, window, cx| lanes_press(this, &owner, node_id.clone(), Pick::Node(node_id.clone()), window, cx))
        })
        .child(mark_view)
        .child(words)
        .children(status);
    let body = labelled(body.aria_selected(picked), Role::Button, name);
    let changed = node.changed.as_ref().map(|changed| {
        // A state the Job's own copy changed: the comparison's glyph in a ring on its corner.
        div()
            .id(SharedString::from(format!("lane-changed:{}", node.id)))
            .absolute()
            .left(px(at.0) + px(rect.w * z) - s(token::SPACE_6) + s(token::SPACE_2))
            .top(px(at.1) - s(token::SPACE_2))
            .size(s(token::SPACE_6))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::STATUS_AWAITING_REVIEW))
            .bg(look.color(token::BG_RAISED))
            .tooltip(tip(format!("Changed on this Job\n{changed}")))
            .child(zoomed_glyph("git-compare", z, look.color(token::STATUS_AWAITING_REVIEW), 14.))
    });
    // A ring on the corner, `right` in from the node's right edge, beside `changed` where both are there.
    let corner = |key: &str, right: Pixels, frame: Hsla, ink: Hsla, icon: &'static str, said: String| {
        labelled(
            div()
                .id(SharedString::from(format!("lane-{key}:{}", node.id)))
                .absolute()
                .left(px(at.0) + px(rect.w * z) - s(token::SPACE_6) - right)
                .top(px(at.1) - s(token::SPACE_2))
                .size(s(token::SPACE_6))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(frame)
                .bg(look.color(token::BG_RAISED))
                .tooltip(tip(said.clone()))
                .child(zoomed_glyph(icon, z, ink, 14.)),
            Role::Image,
            said,
        )
    };
    let alone = -s(token::SPACE_2);
    // The places that set this state otherwise: neutral, as nothing is wrong with it.
    let differs = (!node.differs.is_empty()).then(|| {
        let right = if node.changed.is_some() { s(token::SPACE_6) - s(token::SPACE_1) } else { alone };
        corner("differs", right, look.color(token::BORDER_STRONG), look.color(token::FG_MUTED), "split", differs_said(&node.differs))
    });
    // Workspaces that tie for the repository set it differently: the caution hue.
    let clash = node.clash.as_ref().map(|clash| {
        let right = if node.changed.is_some() || !node.differs.is_empty() { s(token::SPACE_6) * 2. - s(token::SPACE_2) } else { alone };
        corner("clash", right, look.color(token::NOTICE_CAUTION), look.color(token::NOTICE_CAUTION), "triangle-alert", clash.clone())
    });
    div().children(ring).child(body).children(changed).children(differs).children(clash).into_any_element()
}

/// One rule's chip on its edge: a diamond wearing its status, its name and when or what it waits for (`RuleEdge`).
#[allow(clippy::too_many_arguments)]
pub(super) fn lane_chip<H: LanesHost>(id: &SharedString, edge: &LaneEdge, target: &str, centre: (f32, f32), wide: f32, z: f32, mine: bool, picked: bool, look: &Look, cx: &Context<H>) -> AnyElement {
    let s = |token| look.length(token) * z;
    let (hue, dashed) = rule_hue(look, edge, false);
    let mark = chip_mark(edge);
    let meta = meta(edge);
    let mut lines = vec![edge.label.clone()];
    if edge.did.is_empty() {
        lines.extend(mark.map(|(_, said)| said.to_string()));
    } else {
        lines.extend(edge.did.iter().cloned());
    }
    lines.extend(meta.clone());
    if let Some(changed) = &edge.changed {
        lines.push("Changed on this Job".to_string());
        lines.push(changed.clone());
    }
    if !edge.differs.is_empty() {
        lines.push(differs_said(&edge.differs));
    }
    lines.extend(edge.clash.clone());
    let proposed = if edge.proposed { "Proposed: " } else { "" };
    let name = if edge.quiet { format!("{proposed}{}, {target}", edge.label) } else { format!("{proposed}{}", edge.label) };
    let quiet = edge.quiet;
    let fill = if quiet { look.color(token::BG_RAISED) } else { look.color(token::BG_OVERLAY) };
    let stroke = mix(hue, look.color(token::BORDER_DEFAULT), 0.7);
    let line_w = look.length(token::BORDER_WIDTH) * 1.5;
    let dash = dashed.then(|| f32::from(look.length(token::SPACE_1) * z));
    let diamond = div()
        .relative()
        .flex_none()
        .size(s(token::SPACE_6))
        .flex()
        .items_center()
        .justify_center()
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let (x, y, w, h) = (bounds.origin.x, bounds.origin.y, bounds.size.width, bounds.size.height);
                    let p = |fx: f32, fy: f32| point(x + w * fx, y + h * fy);
                    paint_polygon(window, &[p(0.5, 1. / 24.), p(23. / 24., 0.5), p(0.5, 23. / 24.), p(1. / 24., 0.5)], fill, stroke, line_w, dash);
                },
            )
            .absolute()
            .size_full(),
        )
        .children(mark.map(|(icon, _)| zoomed_glyph(icon, z, hue, 12.)));
    let chip = div()
        .id(SharedString::from(format!("lane-chip:{}", edge.id)))
        .flex()
        .items_center()
        .gap(s(token::SPACE_1))
        .h(px(CHIP_H * z))
        .when(quiet, |chip| chip.w(px(CHIP_H * z)).justify_center())
        .when(!quiet, |chip| chip.pl(s(token::SPACE_1)).pr(s(token::SPACE_2)).bg(look.color(token::BG_OVERLAY)))
        .rounded(s(token::RADIUS_SM))
        .text_size(s(token::TEXT_XS))
        .line_height(s(token::LEADING_XS))
        .text_color(if edge.autonomy == LaneAutonomy::Off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
        .whitespace_nowrap()
        .cursor_pointer()
        .when(!quiet, |chip| chip.hover(|style| style.bg(look.color(token::BG_HOVER))))
        .when(picked, |chip| {
            chip.bg(select_ground(look))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .shadow(vec![gpui::BoxShadow { color: select_edge(look), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: true }])
        })
        .when(mine, |chip| chip.border(look.length(token::BORDER_WIDTH) * 2.).border_color(look.color(token::ACCENT)))
        .tooltip(tip(lines.join("\n")))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click({
            let (owner, edge_id, rule) = (id.clone(), edge.id.clone(), edge.rule.clone());
            cx.listener(move |this: &mut H, _, window, cx| lanes_press(this, &owner, edge_id.clone(), Pick::Rule(rule.clone()), window, cx))
        })
        .child(diamond)
        .when(!quiet, |chip| chip.child(SharedString::from(edge.label.clone())).children(meta.map(|meta| div().font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta)))))
        .children(edge.changed.as_ref().map(|_| zoomed_glyph("git-compare", z, look.color(token::STATUS_AWAITING_REVIEW), 14.)))
        .children((!edge.differs.is_empty()).then(|| zoomed_glyph("split", z, look.color(token::FG_MUTED), 14.)))
        .children(edge.clash.as_ref().map(|clash| {
            labelled(div().id(SharedString::from(format!("lane-chip-clash:{}", edge.id))).flex_none().flex().child(zoomed_glyph("triangle-alert", z, look.color(token::NOTICE_CAUTION), 14.)), Role::Image, clash.clone())
        }));
    // Centred on its place on the edge, whatever its words make its width.
    let room = wide.max(CHIP_H * z) * 2.;
    div()
        .absolute()
        .left(px(centre.0 - room / 2.))
        .top(px(centre.1) - px(CHIP_H * z) / 2.)
        .w(px(room))
        .flex()
        .justify_center()
        .child(labelled(chip.aria_selected(picked), Role::Button, name))
        .into_any_element()

}
