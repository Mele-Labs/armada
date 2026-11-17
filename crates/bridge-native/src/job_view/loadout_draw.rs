//! The Loadout tab drawn: the source filter, the row a press opened, then Knows, Told and Used by turn, as Classic's
//! `LoadoutSheet`, `LoadoutMcp`, `LoadoutSkill` and `LoadoutItemPanel` lay them out. Every row's kind wears its glyph and
//! hue, a state is its registry glyph with a tooltip, and the acts down a list sit in one column of one width.

use gpui::{div, prelude::*, relative, Anchor, AnyElement, Context, DismissEvent, Div, Focusable, FontWeight, Hsla, Role, SharedString, Stateful, Window};
use gpui_component::text::TextView;
use ipc::{LoadoutItem, LoadoutItemBody, LoadoutItemFormat, LoadoutKind, LoadoutTurn, LoadoutUse, SessionMcpServer};

use super::loadout_model::{self as model, ItemRef, Mark, SkillAct};
use super::{Body, LoadoutTab, Read};
use crate::job_view::head::{below_left, drawn, tip};
use crate::job_view::rows::{chevron, glyph, mono, Size};
use crate::job_view::tabs::frame;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// A menu's ids live as long as the app: the source filter's rows by place.
const SOURCE_IDS: [&str; 24] = [
    "source-0", "source-1", "source-2", "source-3", "source-4", "source-5", "source-6", "source-7", "source-8", "source-9", "source-10", "source-11", "source-12", "source-13",
    "source-14", "source-15", "source-16", "source-17", "source-18", "source-19", "source-20", "source-21", "source-22", "source-23",
];

impl Render for LoadoutTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let list = div()
            .id("loadout-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .px(look.length(token::SPACE_3))
            .pt(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_4))
            .flex()
            .flex_col()
            .children(self.sections(look, cx));
        labelled(
            div()
                .id("loadout")
                .track_focus(&self.focus)
                .size_full()
                .flex()
                .flex_col()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .child(self.head(look, cx))
                .children(self.item_panel(look, cx))
                .child(list)
                .children(self.menu.as_ref().map(|(menu, _)| menu.clone().into_any_element())),
            Role::Group,
            "Loadout",
        )
    }
}

impl LoadoutTab {
    // ---- the filter --------------------------------------------------------------------------

    fn head(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said: SharedString = self.source.clone().unwrap_or_else(|| "All sources".into()).into();
        div()
            .flex_none()
            .flex()
            .items_center()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_b_1()
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(
                control(look, "loadout-source".into(), true)
                    .rounded(look.length(token::RADIUS_MD))
                    .gap(look.length(token::SPACE_1))
                    .child(said)
                    .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_MUTED)))
                    .tooltip(tip("Source".into()))
                    .on_click(cx.listener(|this, _, window, cx| this.open_sources(window, cx))),
                Role::ComboBox,
                "Source",
            ))
            .into_any_element()
    }

    /// Every source a row names, each once, with All sources first.
    fn sources(&self) -> Vec<String> {
        let turns = self.turns();
        let uses: Vec<&LoadoutUse> = turns.iter().flat_map(|turn| turn.used.iter()).collect();
        model::sources_of(self.items(), &uses)
    }

    fn open_sources(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sources = self.sources();
        let picked = self.source.clone();
        let entries = std::iter::once(Entry::Choice { id: SOURCE_IDS[0], label: "All sources".into(), on: picked.is_none() })
            .chain(sources.iter().zip(SOURCE_IDS.iter().skip(1)).map(|(one, id)| Entry::Choice { id, label: one.clone().into(), on: picked.as_deref() == Some(one.as_str()) }))
            .collect();
        self.open_menu("Source", entries, (Role::ComboBox, "Source".into()), window, cx, move |this, id, _, cx| {
            let at = SOURCE_IDS.iter().position(|one| *one == id).unwrap_or(0);
            this.pick_source(at.checked_sub(1).and_then(|at| sources.get(at).cloned()), cx);
        });
    }

    /// A menu under the control named `from`, each pick handed to `picked`, and the keys back where they were when it goes.
    fn open_menu(
        &mut self,
        name: &str,
        entries: Vec<Entry>,
        from: (Role, String),
        window: &mut Window,
        cx: &mut Context<Self>,
        picked: impl Fn(&mut Self, &'static str, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let at = drawn(window, cx, from.0, &from.1).map_or_else(|| window.mouse_position(), below_left);
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new(name.to_string(), entries, at, Anchor::TopLeft, false, cx));
        let pick = cx.subscribe_in(&menu, window, move |this, _, event: &Picked, window, cx| {
            let Picked::Entry { id, .. } = event;
            this.menu = None;
            picked(this, id, window, cx);
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some((menu, vec![pick, dismissed]));
        cx.notify();
    }

    // ---- the list ----------------------------------------------------------------------------

    /// The list's children, flat, so `/mcp` can bring the MCP servers' group to the top by its place (`mcp_at`).
    fn sections(&self, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut out = Vec::new();
        if matches!(self.read, Read::Failed) {
            out.push(none(look, "Not read. Close the tab and open it again."));
        }
        let known = self.held_known();
        let reading = matches!(self.read, Read::Reading);
        out.push(heading(look, "Knows", false, !reading && known.is_empty()));
        out.extend(known.iter().map(|kind| self.kind_group(*kind, look, cx)));
        if reading {
            out.push(skeleton(look));
        }
        if let Read::Read(_) = self.read {
            let told: Vec<LoadoutKind> = model::TOLD.into_iter().filter(|kind| self.kept(*kind).next().is_some()).collect();
            out.push(heading(look, "Told", true, told.is_empty()));
            out.extend(told.iter().map(|kind| self.kind_group(*kind, look, cx)));
        }
        out.push(self.turns_section(look, cx));
        out
    }

    fn kind_group(&self, kind: LoadoutKind, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = model::kind_said(kind);
        let group = div().flex().flex_col().gap(look.length(token::SPACE_1)).pt(look.length(token::SPACE_3)).child(eyebrow(look, kind));
        let body = match (kind, self.live_servers()) {
            (LoadoutKind::McpServer, Some(servers)) if self.hosted => group
                .children(self.mcp.said.clone().map(|said| none(look, &said)))
                .child(rows(look).children(servers.into_iter().map(|server| self.server_row(server, look, cx)))),
            _ => {
                let items: Vec<&LoadoutItem> = self.kept(kind).collect();
                let whole = self.whole.contains(&kind) || items.len() <= model::BOUND;
                let more = items.len().saturating_sub(model::BOUND);
                let acts = kind == LoadoutKind::Skill && items.iter().any(|one| self.skill_acts(&ItemRef::of(one).key()).is_some());
                group
                    .when(kind == LoadoutKind::McpServer, |group| group.children(self.mcp.said.clone().map(|said| none(look, &said))))
                    .child(rows(look).children(items.iter().take(if whole { items.len() } else { model::BOUND }).map(|one| self.item_row(one, acts, look, cx))))
                    .when(!whole, |group| {
                        let words = format!("Show {more} more");
                        group.child(labelled(
                            control(look, SharedString::from(format!("loadout-more:{said}")), false)
                                .rounded(look.length(token::RADIUS_MD))
                                .child(SharedString::from(words.clone()))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.whole.push(kind);
                                    cx.notify();
                                })),
                            Role::Button,
                            format!("{words} {said}"),
                        ))
                    })
            }
        };
        labelled(div().id(SharedString::from(format!("loadout-group:{said}"))).child(body), Role::Group, said).into_any_element()
    }

    /// A row the agent reported, which a press opens, with a skill's acts where Kit holds it.
    fn item_row(&self, item: &LoadoutItem, acts: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let reference = ItemRef::of(item);
        let key = reference.key();
        let mark = if self.gone.contains_key(&key) { Some(Mark::Removed) } else { Mark::of_report(item.state) };
        let source = model::source_said(item.source, item.from.as_deref());
        let press = self.press(&reference, look, cx).child(name_cell(look, &item.name)).child(mark_cell(look, &key, mark)).child(said_cell(look, source));
        let column = acts.then(|| self.skill_cell(&key, &item.name, look, cx));
        let refused = acts.then(|| self.skill_refused.get(&key).map(|said| refusal(look, said))).flatten();
        acted_row(look, labelled(press, Role::Button, item.name.clone()), column, refused)
    }

    /// The part of a row a press opens: its kind's glyph first.
    fn press(&self, reference: &ItemRef, look: &Look, cx: &Context<Self>) -> Stateful<Div> {
        let open = self.opened.as_ref().is_some_and(|opened| opened.item == *reference);
        let (icon, hue) = model::kind_look(reference.kind);
        let target = reference.clone();
        let hover = look.color(token::BG_HOVER);
        let ring = look.color(token::ACCENT);
        div()
            .id(SharedString::from(format!("loadout-row:{}", reference.key())))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_SM))
            .cursor_pointer()
            .tab_index(0)
            .focus_visible(move |style| style.border_1().border_color(ring))
            .hover(move |style| style.bg(hover))
            .when(open, |row| row.bg(hover))
            .child(glyph(look, icon, Size::Small, look.color(hue)))
            .on_click(cx.listener(move |this, _, _, cx| this.open_item(target.clone(), cx)))
    }

    /// A live server: its name, its state's mark, what the CLI said of it, and its acts.
    fn server_row(&self, server: &SessionMcpServer, look: &Look, cx: &Context<Self>) -> AnyElement {
        let reference = model::mcp_ref_of(&server.name, self.items());
        let key = reference.key();
        let press = labelled(
            self.press(&reference, look, cx).child(name_cell(look, &server.name)).child(mark_cell(look, &key, Some(Mark::of_live(server.state)))).child(said_cell(look, model::server_said(server))),
            Role::Button,
            server.name.clone(),
        );
        let (face_act, menu, pending) = self.server_acts(server);
        let refused = self.server_refused(server);
        let name = server.name.clone();
        let column = acts_cell(look).children(face_act.map(|act| {
            let words = model::act_said(act);
            let word = if pending { words.1 } else { words.0 };
            let face_id = SharedString::from(format!("mcp-face:{name}"));
            let caret_id = SharedString::from(format!("mcp-more:{name}"));
            let on_face = name.clone();
            let face_button = labelled(
                face(look, face_id, word, pending, !menu.is_empty(), refused.is_some()).when(!pending, |button| {
                    button.on_click(cx.listener(move |this, _, _, cx| this.mcp_act(on_face.clone(), act, cx)))
                }),
                Role::Button,
                format!("{word} {name}"),
            );
            let caret_button = (!menu.is_empty()).then(|| {
                let (more, menu, on_menu) = (format!("More for {name}"), menu.clone(), name.clone());
                let from = more.clone();
                labelled(
                    caret(look, caret_id, pending).when(!pending, |button| {
                        button.on_click(cx.listener(move |this, _, window, cx| {
                            let entries = menu.iter().map(|act| Entry::Item { id: model::act_id(*act), label: model::act_said(*act).0.into(), icon: None, danger: false, key: None, off: None }).collect();
                            let server = on_menu.clone();
                            this.open_menu(&from, entries, (Role::Button, from.clone()), window, cx, move |this, id, _, cx| {
                                if let Some(act) = model::act_of_id(id) {
                                    this.mcp_act(server.clone(), act, cx);
                                }
                            });
                        }))
                    }),
                    Role::Button,
                    more,
                )
            });
            div().flex().items_center().child(face_button).children(caret_button)
        }));
        acted_row(look, press.into_any_element(), Some(column.into_any_element()), refused.map(|said| refusal(look, said)))
    }

    /// A skill row's acts: Edit on its face and Remove behind its caret, or Restore once removed, or the word of an act out.
    fn skill_cell(&self, key: &str, name: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some((face_act, menu, pending)) = self.skill_acts(key) else { return acts_cell(look).into_any_element() };
        let refused = self.skill_refused.contains_key(key);
        acts_cell(look)
            .children(face_act.map(|act| {
                let word = match act {
                    SkillAct::Edit => "Edit",
                    SkillAct::Restore if !pending => "Restore",
                    _ => model::skill_underway(act),
                };
                let (on_face, on_name) = (key.to_string(), name.to_string());
                let face_button = labelled(
                    face(look, SharedString::from(format!("skill-face:{key}")), word, pending, !menu.is_empty(), refused).when(!pending, |button| {
                        button.on_click(cx.listener(move |this, _, window, cx| this.skill_act(on_face.clone(), on_name.clone(), act, window, cx)))
                    }),
                    Role::Button,
                    format!("{word} {name}"),
                );
                let plugin = self.matched.get(key).and_then(|found| found.acts.plugin.as_ref().map(|plugin| plugin.name.clone()));
                let caret_button = (!menu.is_empty()).then(|| {
                    let more = format!("More for {name}");
                    let (from, menu, on_key, on_name) = (more.clone(), menu.clone(), key.to_string(), name.to_string());
                    labelled(
                        caret(look, SharedString::from(format!("skill-more:{key}")), pending).on_click(cx.listener(move |this, _, window, cx| {
                            let entries = menu
                                .iter()
                                .map(|act| match act {
                                    SkillAct::RemovePlugin => Entry::Item {
                                        id: "remove_plugin",
                                        label: format!("Remove plugin {}", plugin.clone().unwrap_or_default()).into(),
                                        icon: None,
                                        danger: true,
                                        key: None,
                                        off: None,
                                    },
                                    _ => Entry::Item { id: "remove", label: "Remove".into(), icon: None, danger: true, key: None, off: None },
                                })
                                .collect();
                            let (key, name) = (on_key.clone(), on_name.clone());
                            this.open_menu(&from, entries, (Role::Button, from.clone()), window, cx, move |this, id, window, cx| {
                                let act = if id == "remove_plugin" { SkillAct::RemovePlugin } else { SkillAct::Remove };
                                this.skill_act(key.clone(), name.clone(), act, window, cx);
                            });
                        })),
                        Role::Button,
                        more,
                    )
                });
                div().flex().items_center().child(face_button).children(caret_button)
            }))
            .into_any_element()
    }

    // ---- the turns ---------------------------------------------------------------------------

    /// The turns, latest first, one open at a time, each listing what it used.
    fn turns_section(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let turns = self.turns();
        let mut section = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(heading(look, "Used by turn", true, turns.is_empty()));
        for turn in turns {
            section = section.child(self.turn(turn, look, cx));
        }
        labelled(div().id("loadout-turns").child(section), Role::Group, "Used by turn").into_any_element()
    }

    fn turn(&self, turn: &LoadoutTurn, look: &Look, cx: &Context<Self>) -> AnyElement {
        let at = turn.started_at.as_str().to_string();
        let open = self.open_turn.as_deref() == Some(at.as_str());
        let stamp = crate::settings::time::stamp_at(&at).unwrap_or_else(|| at.clone());
        let said = format!("Turn at {stamp}");
        let pressed = at.clone();
        let hover = look.color(token::BG_HOVER);
        let ring = look.color(token::ACCENT);
        let head = labelled(
            div()
                .id(SharedString::from(format!("loadout-turn:{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(move |style| style.border_1().border_color(ring))
                .hover(move |style| style.bg(hover))
                .when(open, |head| head.bg(hover))
                .child(chevron(look, open))
                .child(SharedString::from(said.clone()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_turn = if this.open_turn.as_deref() == Some(pressed.as_str()) { None } else { Some(pressed.clone()) };
                    cx.notify();
                })),
            Role::Button,
            said,
        );
        let used: Vec<&LoadoutUse> = turn.used.iter().filter(|one| self.shown(&model::source_said(one.source, one.from.as_deref()))).collect();
        let body = open.then(|| {
            let inner = div().pl(look.length(token::SPACE_6));
            if used.is_empty() {
                inner.child(none(look, "None from this source")).into_any_element()
            } else {
                inner.child(rows(look).children(used.into_iter().map(|one| self.use_row(one, look, cx)))).into_any_element()
            }
        });
        div().flex().flex_col().pb(look.length(token::SPACE_2)).child(head).children(body).into_any_element()
    }

    fn use_row(&self, one: &LoadoutUse, look: &Look, cx: &Context<Self>) -> AnyElement {
        let reference = ItemRef::of_use(one);
        let said = match one.by.as_deref() {
            Some(by) => format!("{} · by {by}", model::use_said(one)),
            None => model::use_said(one),
        };
        let muted = look.color(token::FG_MUTED);
        let press = self
            .press(&reference, look, cx)
            .child(div().flex_none().text_color(muted).child(model::use_kind_said(one.kind)))
            .child(name_cell(look, &one.name))
            .children(model::count_said(one).map(|count| div().flex_none().text_color(muted).child(SharedString::from(count))))
            .child(said_cell(look, said));
        acted_row(look, labelled(press, Role::Button, one.name.clone()).into_any_element(), None, None)
    }

    // ---- the row opened ----------------------------------------------------------------------

    /// What a press opened, over the list, with a close.
    fn item_panel(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let opened = self.opened.as_ref()?;
        let item = &opened.item;
        let (icon, hue) = model::kind_look(item.kind);
        let key = item.key();
        let path = match &opened.body {
            Body::Read(LoadoutItemBody::Read { path: Some(path), .. }) => Some(path.clone()),
            _ => None,
        };
        let mark = match item.kind {
            LoadoutKind::McpServer => self.server(&item.name).map(|one| Mark::of_live(one.state)),
            _ if self.gone.contains_key(&key) => Some(Mark::Removed),
            _ => self.items().iter().find(|one| ItemRef::of(one) == *item).and_then(|one| Mark::of_report(one.state)),
        };
        let close = labelled(
            frame(look, "loadout-close", "x").tooltip(tip("Close".into())).on_click(cx.listener(|this, _, _, cx| this.close_item(cx))),
            Role::Button,
            format!("Close {}", item.name),
        );
        let title = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(glyph(look, icon, Size::Large, look.color(hue)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .font_family(mono(look))
                            .child(SharedString::from(item.name.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .text_color(look.color(token::FG_MUTED))
                            .child(SharedString::from(model::source_said(item.source, item.from.as_deref())))
                            .children(mark.map(|mark| mark_cell(look, &format!("panel:{key}"), Some(mark))))
                            .children(path.map(|path| div().min_w_0().font_family(mono(look)).child(SharedString::from(path)))),
                    ),
            )
            .child(close);
        let body = match &opened.body {
            Body::Server => self.server_tools(item, look),
            Body::Reading => skeleton(look),
            Body::Failed => none(look, "Not read. Press the row again."),
            Body::Read(LoadoutItemBody::NotRead { why }) => div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(SharedString::from(why.clone())).into_any_element(),
            Body::Read(LoadoutItemBody::Read { format, body, truncated, .. }) => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .when(*truncated, |body| body.child(none(look, "Only the start is shown.")))
                .child(match format {
                    LoadoutItemFormat::Markdown => TextView::markdown(gpui::ElementId::Name(format!("loadout-read:{key}").into()), model::fenced_front_matter(body)).w_full().selectable(true).into_any_element(),
                    LoadoutItemFormat::Text => div().font_family(mono(look)).child(SharedString::from(body.clone())).into_any_element(),
                })
                .into_any_element(),
        };
        Some(
            labelled(
                div()
                    .id("loadout-item")
                    .flex_none()
                    .max_h(relative(0.5))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_3))
                    .bg(look.color(token::BG_RAISED))
                    .border_b_1()
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(title)
                    .child(body),
                Role::Region,
                item.name.clone(),
            )
            .into_any_element(),
        )
    }

    /// An MCP server's tools, from the Loadout, Loaded or Deferred.
    fn server_tools(&self, server: &ItemRef, look: &Look) -> AnyElement {
        let tools = model::tools_of(server, self.items());
        let (icon, hue) = model::kind_look(LoadoutKind::Tool);
        let list = if tools.is_empty() {
            none(look, "None reported")
        } else {
            rows(look)
                .children(tools.into_iter().map(|one| {
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .border_b_1()
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(glyph(look, icon, Size::Small, look.color(hue)))
                        .child(name_cell(look, &one.name))
                        .child(said_cell(look, model::tool_state_said(one.state).to_string()))
                }))
                .into_any_element()
        };
        div().flex().flex_col().gap(look.length(token::SPACE_1)).child(eyebrow_words(look, "Tools")).child(list).into_any_element()
    }
}

// ---- pieces -------------------------------------------------------------------------------------

/// A section's heading, with None reported under it where it holds nothing.
fn heading(look: &Look, words: &'static str, gap: bool, empty: bool) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("loadout-heading:{words}")))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .when(gap, |heading| heading.pt(look.length(token::SPACE_6)))
            .child(
                div()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(words),
            )
            .when(empty, |heading| heading.child(none(look, "None reported"))),
        Role::Heading,
        words,
    )
    .into_any_element()
}

/// A kind's eyebrow: its glyph in its hue, and its words.
fn eyebrow(look: &Look, kind: LoadoutKind) -> AnyElement {
    let (icon, hue) = model::kind_look(kind);
    div().flex().items_center().gap(look.length(token::SPACE_1)).child(glyph(look, icon, Size::Small, look.color(hue))).child(eyebrow_words(look, model::kind_said(kind))).into_any_element()
}

fn eyebrow_words(look: &Look, words: &'static str) -> AnyElement {
    div()
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_MUTED))
        .child(words)
        .into_any_element()
}

fn none(look: &Look, words: &str) -> AnyElement {
    div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(words.to_string())).into_any_element()
}

fn rows(_: &Look) -> Div {
    div().flex().flex_col()
}

/// A row and, where it has them, its acts in their column and a refusal under both.
fn acted_row(look: &Look, press: impl IntoElement, acts: Option<AnyElement>, refused: Option<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(press).children(acts))
        .children(refused)
        .into_any_element()
}

fn name_cell(look: &Look, name: &str) -> AnyElement {
    div().flex_1().min_w_0().font_family(mono(look)).child(SharedString::from(name.to_string())).into_any_element()
}

fn said_cell(look: &Look, said: String) -> AnyElement {
    div().flex_none().max_w(relative(0.5)).text_color(look.color(token::FG_MUTED)).text_right().child(SharedString::from(said)).into_any_element()
}

/// The state's column: its glyph in its hue with its tooltip, or nothing, at one width down the list.
fn mark_cell(look: &Look, key: &str, mark: Option<Mark>) -> AnyElement {
    let cell = div().flex_none().w(look.length(token::SPACE_4)).flex().justify_center();
    match mark {
        None => cell.into_any_element(),
        Some(mark) => {
            let (icon, said, hue) = mark.look();
            cell.child(labelled(div().id(SharedString::from(format!("loadout-mark:{key}"))).tooltip(tip(said.into())).child(glyph(look, icon, Size::Small, look.color(hue))), Role::Image, said))
                .into_any_element()
        }
    }
}

/// The acts' column: one width on every row of a list, so the buttons stand in a line.
fn acts_cell(look: &Look) -> Div {
    div().flex_none().w(look.length(token::H_CONTROL) * 4.).flex().justify_end()
}

fn refusal(look: &Look, said: &str) -> AnyElement {
    div().px(look.length(token::SPACE_2)).pb(look.length(token::SPACE_1)).text_color(look.color(token::ERROR)).child(SharedString::from(said.to_string())).into_any_element()
}

/// A small bordered control, which Enter and Space press while it holds the keys.
fn control(look: &Look, id: SharedString, live: bool) -> Stateful<Div> {
    let (hover, ring) = (look.color(token::BG_HOVER), look.color(token::ACCENT));
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .border_1()
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_RAISED))
        .tab_index(0)
        .focus_visible(move |style| style.border_color(ring))
        .when(live, |control| control.cursor_pointer().hover(move |style| style.bg(hover)))
}

/// An act's face: its word, quiet while it is out, ringed in the error hue where it was refused.
fn face(look: &Look, id: SharedString, word: &'static str, pending: bool, joined: bool, refused: bool) -> Stateful<Div> {
    let radius = look.length(token::RADIUS_MD);
    let ink: Hsla = if pending { look.color(token::FG_MUTED) } else { look.color(token::FG_DEFAULT) };
    control(look, id, !pending)
        .text_color(ink)
        .when(joined, |face| face.rounded_tl(radius).rounded_bl(radius))
        .when(!joined, |face| face.rounded(radius))
        .when(refused, |face| face.border_color(look.color(token::ERROR)))
        .child(word)
}

/// The caret beside a face, which opens the rest of the acts.
fn caret(look: &Look, id: SharedString, pending: bool) -> Stateful<Div> {
    let radius = look.length(token::RADIUS_MD);
    control(look, id, !pending).px(look.length(token::SPACE_1)).border_l_0().rounded_tr(radius).rounded_br(radius).child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_MUTED)))
}

/// Bars that pulse where what is being read will stand.
fn skeleton(look: &Look) -> AnyElement {
    use gpui::{Animation, AnimationExt};
    let widths = [8., 6., 7., 5.];
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .pt(look.length(token::SPACE_3))
        .children(widths.into_iter().enumerate().map(|(at, width)| {
            div()
                .h(look.length(token::SPACE_3))
                .w(look.length(token::SPACE_6) * width)
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::FG_DEFAULT).opacity(0.06))
                .with_animation(
                    SharedString::from(format!("loadout-skeleton-{at}")),
                    Animation::new(look.duration(token::DURATION_PULSE) * 2).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                    |bar, at| bar.opacity(at),
                )
        }))
        .into_any_element()
}
