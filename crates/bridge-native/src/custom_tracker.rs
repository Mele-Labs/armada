//! The Custom choice of a tracker. Picking it opens a name field in place under the tracker row, and Make its mod
//! starts a Job that makes a tracker mod for that tracker (the armada-mods skill, *Making a tracker mod*). Settings
//! (Starting Jobs) and the New project sheet share it: each host starts the Job through its own wire and hands
//! the answer back.

use gpui::{div, prelude::*, AnyElement, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Render, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{ModKind, ModOrigin, ModSummary};

use crate::keymap::TEXT_BOX;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, tip, Btn, Weight};

/// The word Custom carries among a tracker's words. It is never saved: a provider is a word Fleet has a tracker for or
/// the name of a tracker mod, and Custom is neither.
pub const CUSTOM: &str = "<custom>";

/// The trackers this build has, by the word the project's `provider` takes (none for the forge's own).
const BUILT_IN: [(&str, &str); 3] = [("", "GitHub Issues"), ("linear", "Linear"), ("beads", "Beads")];

/// Every word the tracker choice offers: the built in ones, then each tracker mod this Mac has that Fleet will use (it is
/// on, valid, and its last try of the file as it stands passed), then Custom.
pub fn options(mods: &[ModSummary]) -> Vec<(String, String)> {
    let mut words: Vec<(String, String)> = BUILT_IN.iter().map(|(word, said)| (word.to_string(), said.to_string())).collect();
    let usable = |one: &&ModSummary| {
        one.kind == Some(ModKind::Tracker) && one.origin == ModOrigin::Local && one.enabled && one.valid && one.tried.as_ref().is_some_and(|tried| tried.passed)
    };
    for one in mods.iter().filter(usable) {
        if !words.iter().any(|(word, _)| *word == one.name) {
            words.push((one.name.clone(), one.name.clone()));
        }
    }
    words.push((CUSTOM.to_string(), "Custom".to_string()));
    words
}

/// The Job's first message: the tracker, the project and the repository, what to follow, and what to leave alone. Plain
/// facts, so the Job needs nothing else to start.
pub fn message(tracker: &str, project: &str, repository: &str) -> String {
    format!(
        "Make a tracker mod for {tracker}.\n\n\
         Project: {project}. Repository: {repository}.\n\
         The project's issues are in {tracker}, and Armada has no tracker for it yet.\n\n\
         Follow the armada-mods skill, the section on a tracker mod. Find how {tracker} is read: its command line, its HTTP API or an MCP server. \
         Make the mod with scaffold_mod (kind tracker, your session_id), write its tracker.toml, check it with validate_mod, and try it against {repository} with try_tracker_mod until a try passes.\n\n\
         The mod's name must not be github, jira, linear or beads, which Fleet reads as built in trackers. If {tracker} is one of those, say so and say what the Manifest needs, and make no mod.\n\
         Do not change the project's tracker setting. When a try passes, say so: the mod is then in the tracker choice under Starting Jobs in the project's settings."
    )
}

/// The dispatch the press sends: the Job starts in `manifest_id`'s repository, in `project` where there is one, and
/// `words` are its request and its first message, as Bridge's own start sends them (`dispatch/screen.rs`, `start_job`).
pub fn dispatch_body(manifest_id: &str, project: Option<&str>, words: &str) -> Result<String, String> {
    let start = ipc::StartSession {
        manifest_id: ipc::ManifestId::carried(manifest_id),
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some(words.to_string()),
        held: false,
        after: None,
        project_rows: Vec::new(),
        on_your_words: None,
        project: project.map(str::to_string),
    };
    let then = vec![ipc::DispatchThen::Message(ipc::DispatchMessage { text: words.to_string(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
    ipc::encode(&ipc::DispatchJob { start, kind: None, then }).map_err(|why| format!("{why:?}"))
}

/// Send it by the route and the client Bridge's own start uses. The answer is the Job's id, none where the Operator holds
/// the idea, or Fleet's sentence.
pub fn send(port: u16, body: String, cx: &gpui::App) -> gpui::Task<Result<Option<String>, String>> {
    cx.background_spawn(async move {
        let bytes = crate::dispatch::live::call(port, "/sessions/dispatch".into(), Some(body))?;
        let done = ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())?;
        Ok(done.session.map(|record| record.id.as_str().to_string()))
    })
}

/// What the field asks of its host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Make its mod, for the tracker named.
    Make(String),
    /// Open the Job that was started, by its id.
    OpenJob(String),
}

enum Phase {
    Asking,
    Starting,
    /// The Job started, or the Operator holds the idea (`None`).
    Started { job: Option<String>, named: SharedString },
    Refused(SharedString),
}

pub struct CustomTracker {
    id: SharedString,
    keep: &'static str,
    input: Entity<InputState>,
    make: FocusHandle,
    open: FocusHandle,
    shown: bool,
    phase: Phase,
    /// Where the Job will start, named in the line under the field.
    repository: SharedString,
    _typing: Vec<Subscription>,
}

impl EventEmitter<Asked> for CustomTracker {}

impl CustomTracker {
    /// `id` names its elements, and `keep` the draft of what is typed (`drafts.rs`).
    pub fn new(id: &'static str, keep: &'static str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Jira"));
        let typing = vec![cx.subscribe_in(&input, window, move |this, state, event: &InputEvent, _, cx| match event {
            InputEvent::Change => {
                crate::drafts::keep(keep, state.read(cx).value().as_ref(), cx);
                if matches!(this.phase, Phase::Refused(_)) {
                    this.phase = Phase::Asking;
                }
                cx.notify();
            }
            InputEvent::PressEnter { secondary: false, .. } => this.make(cx),
            _ => {}
        })];
        CustomTracker {
            id: id.into(),
            keep,
            input,
            make: cx.focus_handle().tab_stop(true),
            open: cx.focus_handle().tab_stop(true),
            shown: false,
            phase: Phase::Asking,
            repository: SharedString::default(),
            _typing: typing,
        }
    }

    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// Whether the field is open and asking, which is where Esc closes it.
    pub fn asking(&self) -> bool {
        self.shown && !matches!(self.phase, Phase::Started { .. })
    }

    /// Open the field, on what was typed last time, with the keys in it.
    pub fn show(&mut self, repository: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.repository = repository;
        if !matches!(self.phase, Phase::Started { .. }) {
            self.phase = Phase::Asking;
        }
        self.shown = true;
        let words = crate::drafts::kept(self.keep, cx).unwrap_or_default();
        if self.input.read(cx).value().is_empty() && !words.is_empty() {
            self.input.update(cx, |input, cx| input.set_value(words, window, cx));
        }
        let input = self.input.clone();
        window.defer(cx, move |window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
        cx.notify();
    }

    /// Shut the field. A Job it started keeps its line.
    pub fn hide(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Started { .. }) {
            self.shown = false;
            self.phase = Phase::Asking;
        }
        cx.notify();
    }

    /// Back to nothing opened, for another project or a Settings opened again.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.shown = false;
        self.phase = Phase::Asking;
        cx.notify();
    }

    /// The tracker's name as typed.
    pub fn name(&self, cx: &gpui::App) -> String {
        self.input.read(cx).value().trim().to_string()
    }

    /// Make its mod: one press asks for one Job, and no second while the first is out.
    pub fn make(&mut self, cx: &mut Context<Self>) {
        let name = self.name(cx);
        if name.is_empty() || matches!(self.phase, Phase::Starting | Phase::Started { .. }) {
            return;
        }
        self.phase = Phase::Starting;
        cx.emit(Asked::Make(name));
        cx.notify();
    }

    /// The Job started: `job` is its id, none where the Operator holds the idea.
    pub fn started(&mut self, job: Option<String>, cx: &mut Context<Self>) {
        let named = SharedString::from(self.name(cx));
        self.phase = Phase::Started { job, named };
        crate::drafts::clear(self.keep, cx);
        cx.notify();
    }

    pub fn refused(&mut self, why: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.phase = Phase::Refused(why.into());
        cx.notify();
    }

    /// Whether any of its keys holds the focus.
    pub fn contains_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        self.stops(cx).iter().any(|stop| stop.contains_focused(window, cx))
    }

    /// The keys' places, in the order Tab reaches them.
    pub fn stops(&self, cx: &gpui::App) -> Vec<FocusHandle> {
        if !self.shown {
            return Vec::new();
        }
        match &self.phase {
            Phase::Started { job: Some(_), .. } => vec![self.open.clone()],
            Phase::Started { job: None, .. } => Vec::new(),
            _ => vec![self.input.focus_handle(cx), self.make.clone()],
        }
    }

    /// What a test types in the field.
    #[cfg(test)]
    pub fn typed(&mut self, words: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.set_value(words.to_string(), window, cx));
    }

    /// The Job the press started, by its id.
    #[cfg(test)]
    pub fn job(&self) -> Option<&str> {
        match &self.phase {
            Phase::Started { job, .. } => job.as_deref(),
            _ => None,
        }
    }

    /// The field with the keys in it.
    pub fn input_focus(&self, cx: &gpui::App) -> FocusHandle {
        self.input.focus_handle(cx)
    }

    fn panel(&self, look: &Look) -> gpui::Div {
        let accent = look.color(token::ACCENT);
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(accent.opacity(0.4))
            .bg(look.color(token::ACCENT_FAINT))
    }

    fn caption(&self, look: &Look, mark: &'static str, said: impl Into<SharedString>) -> gpui::Div {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph(mark, look.length(token::SPACE_4), look.color(token::ACCENT)))
            .child(said.into())
    }

    fn small(look: &Look, said: impl Into<SharedString>) -> gpui::Div {
        div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(said.into())
    }
}

impl Render for CustomTracker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.shown {
            return div().into_any_element();
        }
        let look = cx.look().clone();
        let look = &look;
        if let Phase::Started { job, named } = &self.phase {
            return self.started_line(job.clone(), named.clone(), look, window, cx);
        }
        let accent = look.color(token::ACCENT);
        let typing = self.input.focus_handle(cx).is_focused(window);
        let ready = !self.name(cx).is_empty();
        let sending = matches!(self.phase, Phase::Starting);
        let field = div()
            .id(SharedString::from(format!("{}-custom-field", self.id)))
            .flex_1()
            .min_w_0()
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .flex()
            .items_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if typing { accent } else { look.color(token::BORDER_DEFAULT) })
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .child(div().flex_1().min_w_0().key_context(TEXT_BOX).child(Input::new(&self.input).appearance(false)));
        let press = Btn::new(SharedString::from(format!("{}-custom-make", self.id)), "Make its mod", Weight::Primary)
            .pending(sending)
            .disabled(!ready)
            .draw(look, cx)
            .track_focus(&self.make)
            .when(ready && !sending, |press| {
                press
                    .on_click(cx.listener(|this, _, _, cx| this.make(cx)))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            this.make(cx);
                        }
                    }))
            })
            .tooltip(tip("Starts a Job that makes the mod"));
        let line = if self.repository.is_empty() {
            "Starts a Job that makes a tracker mod for it, then tries the mod.".to_string()
        } else {
            format!("Starts a Job in {} that makes a tracker mod for it, then tries the mod.", self.repository)
        };
        let refused = match &self.phase {
            Phase::Refused(why) => Some(labelled(crate::settings::view::refusal(look, why.clone()).id(SharedString::from(format!("{}-custom-refused", self.id))), Role::Alert, why.clone())),
            _ => None,
        };
        let top = self.caption(look, "puzzle", "Your tracker's name");
        let row = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(labelled(field, Role::TextInput, "Your tracker's name"))
            .child(labelled(press, Role::Button, "Make its mod"));
        self.panel(look)
            .id(SharedString::from(format!("{}-custom", self.id)))
            .child(top)
            .child(row)
            .child(Self::small(look, line))
            .children(refused)
            .into_any_element()
    }
}

impl CustomTracker {
    /// The line a started Job leaves: that it started, a press that opens it, and what stays as it was.
    fn started_line(&self, job: Option<String>, named: SharedString, look: &Look, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let said = if job.is_some() { format!("Job started: a tracker mod for {named}") } else { format!("Sent to the Operator: a tracker mod for {named}") };
        let open = job.map(|job| {
            let ring = crate::settings::view::ring_colour(look);
            let focused = self.open.is_focused(window);
            let press = div()
                .id(SharedString::from(format!("{}-custom-open", self.id)))
                .track_focus(&self.open)
                .flex_none()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if focused { ring } else { gpui::transparent_black() })
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::ACCENT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child("Open the Job")
                .on_click(cx.listener({
                    let job = job.clone();
                    move |_, _, _, cx| cx.emit(Asked::OpenJob(job.clone()))
                }))
                .on_key_down(cx.listener(move |_, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        cx.stop_propagation();
                        cx.emit(Asked::OpenJob(job.clone()));
                    }
                }));
            labelled(press, Role::Button, "Open the Job")
        });
        let status = div()
            .id(SharedString::from(format!("{}-custom-started", self.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph("check", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_SUCCESS)))
            .child(said.clone());
        let head = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_3)).child(labelled(status, Role::Status, said)).children(open);
        self.panel(look)
            .id(SharedString::from(format!("{}-custom", self.id)))
            .child(head)
            .child(Self::small(look, "The choice stays as it was until the mod exists."))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker_mod(name: &str, passed: Option<bool>) -> ModSummary {
        let tried = passed.map(|passed| format!(r#","tried":{{"at":"2026-11-10T08:45:00Z","manifest_id":"armada","passed":{passed},"listed":3}}"#)).unwrap_or_default();
        let said = format!(r#"{{"name":"{name}","kind":"tracker","enabled":true,"valid":true{tried}}}"#);
        ipc::decode("ModSummary", said.as_bytes()).expect("decodes")
    }

    #[test]
    fn the_choice_lists_the_built_in_trackers_then_the_mods_that_pass_a_try_then_custom() {
        let words = options(&[tracker_mod("acme-issues", Some(true)), tracker_mod("untried", None), tracker_mod("failing", Some(false))]);
        let said: Vec<&str> = words.iter().map(|(_, said)| said.as_str()).collect();
        assert_eq!(said, ["GitHub Issues", "Linear", "Beads", "acme-issues", "Custom"]);
    }

    #[test]
    fn a_mod_that_is_off_or_not_a_tracker_is_not_offered() {
        let mut off = tracker_mod("off-board", Some(true));
        off.enabled = false;
        let mut theme = tracker_mod("dusk", Some(true));
        theme.kind = Some(ModKind::Theme);
        let words = options(&[off, theme]);
        assert_eq!(words.len(), 4, "the three built in and Custom");
    }

    #[test]
    fn the_first_message_names_the_tracker_the_project_and_the_repository() {
        let said = message("Jira", "Storefront", "sample-web");
        assert!(said.starts_with("Make a tracker mod for Jira."));
        assert!(said.contains("Project: Storefront. Repository: sample-web."));
        assert!(said.contains("armada-mods skill"));
        assert!(said.contains("try_tracker_mod"));
        assert!(!said.contains('\u{2014}') && !said.contains('\u{2013}'), "no dashes in the copy");
    }
}
