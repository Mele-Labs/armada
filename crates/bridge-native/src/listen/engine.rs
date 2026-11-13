//! `Listener`: the one entity that owns the line of cards and the speaker. Views read cards from it and press
//! its acts; none of them starts the helper. The helper, or the mock in a walk, runs off the main thread and
//! answers over a channel, and a task folds each answer into the line and notifies once.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{Context, EventEmitter, FocusHandle, Task};

use super::speaker::{voice_label, Event, Speaker, Speech, VoiceInfo};
use super::{held, Card, Line, Said};

/// A card came, or one could not be spoken: what the thread draws changed in shape, not only in progress.
pub struct Changed;

/// The two stops of a card's buttons.
#[derive(Clone)]
pub struct Focuses {
    pub disc: FocusHandle,
    pub again: FocusHandle,
}

struct Running {
    run: u64,
    speech: Speech,
    _task: Task<()>,
}

pub struct Listener {
    line: Line,
    speaker: Speaker,
    running: Option<Running>,
    ticker: Option<Task<()>>,
    last_tick: Instant,
    voices: Vec<VoiceInfo>,
    voices_asked: bool,
    focus: HashMap<u64, Focuses>,
    _voices_task: Option<Task<()>>,
}

impl EventEmitter<Changed> for Listener {}

impl Listener {
    pub fn new(speaker: Speaker) -> Listener {
        let (voices, voices_asked) = match &speaker {
            Speaker::Mock(mock) => (mock.voices.clone(), true),
            Speaker::Helper => (Vec::new(), false),
        };
        Listener { line: Line::default(), speaker, running: None, ticker: None, last_tick: Instant::now(), voices, voices_asked, focus: HashMap::new(), _voices_task: None }
    }

    /// The cards of one Job, oldest first.
    pub fn cards_of<'a>(&'a self, job: &'a str) -> impl Iterator<Item = &'a Card> {
        self.line.cards().iter().filter(move |card| card.job == job)
    }

    pub fn card(&self, id: u64) -> Option<&Card> {
        self.line.card(id)
    }

    pub fn focuses(&self, id: u64) -> Option<&Focuses> {
        self.focus.get(&id)
    }

    /// The name of the voice that speaks, where the Mac lists it.
    pub fn voice_name(&self) -> Option<String> {
        voice_label(&self.voices, &held().voice)
    }

    /// Change what the mock speaker does next. Nothing for the real one.
    pub fn mock(&mut self, change: impl FnOnce(&mut super::speaker::Mock)) {
        if let Speaker::Mock(mock) = &mut self.speaker {
            change(mock);
        }
    }

    /// Read something out: it joins the line, and speaks when the line before it ends.
    pub fn say(&mut self, said: Said, cx: &mut Context<Self>) {
        let id = self.line.add(said);
        self.focus.insert(id, Focuses { disc: cx.focus_handle().tab_stop(true), again: cx.focus_handle().tab_stop(true) });
        self.read_voices(cx);
        cx.emit(Changed);
        self.dispatch(cx);
        cx.notify();
    }

    /// The Mac's voices, once, so a card can name the one that speaks.
    fn read_voices(&mut self, cx: &mut Context<Self>) {
        if self.voices_asked || held().voice.is_empty() {
            return;
        }
        self.voices_asked = true;
        self._voices_task = Some(cx.spawn(async move |this, cx| {
            let voices = cx
                .background_executor()
                .spawn(async move { crate::voice::helper().map(|helper| super::speaker::list_voices(&helper)).unwrap_or_default() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.voices = voices;
                cx.notify();
            });
        }));
    }

    /// Start the next card if nothing is speaking, paused or still ending.
    fn dispatch(&mut self, cx: &mut Context<Self>) {
        if self.running.is_some() {
            return;
        }
        let Some((id, run)) = self.line.start_next() else { return };
        let Some(card) = self.line.card(id) else { return };
        let (words, total) = (card.words.clone(), card.total);
        let (speech, ahead) = self.speaker.speak(&words, &held().voice, total);
        self.line.set_played(id, ahead);
        let events = speech.events.clone();
        let task = cx.spawn(async move |this, cx| {
            while let Ok(event) = events.recv().await {
                if !this.update(cx, |this, cx| this.heard(id, run, event, cx)).unwrap_or(false) {
                    break;
                }
            }
        });
        self.running = Some(Running { run, speech, _task: task });
        self.keep_time(cx);
    }

    /// The speaker said something about a start of a card. True while more is to come.
    fn heard(&mut self, id: u64, run: u64, event: Event, cx: &mut Context<Self>) -> bool {
        let more = match event {
            Event::Speaking => {
                self.line.began(id, run);
                true
            }
            Event::Spoken | Event::Stopped => {
                if event == Event::Spoken {
                    self.line.ended(id, run);
                }
                false
            }
            Event::Failed(why) => {
                if self.line.failed(id, run, why) {
                    cx.emit(Changed);
                }
                false
            }
        };
        if !more {
            if self.running.as_ref().is_some_and(|running| running.run == run) {
                self.running = None;
            }
            self.dispatch(cx);
        }
        cx.notify();
        more
    }

    /// While a card speaks its bar moves, a few times a second, and then the clock stops.
    fn keep_time(&mut self, cx: &mut Context<Self>) {
        if self.ticker.is_some() {
            return;
        }
        self.last_tick = Instant::now();
        self.ticker = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(250)).await;
            if !this.update(cx, |this, cx| this.tick(cx)).unwrap_or(false) {
                break;
            }
        }));
    }

    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let now = Instant::now();
        self.line.tick(now - std::mem::replace(&mut self.last_tick, now));
        let speaking = self.line.cards().iter().any(|card| card.state == super::State::Speaking);
        if !speaking {
            self.ticker = None;
        }
        cx.notify();
        speaking
    }

    /// What is speaking stops now and its answers are no longer heard, so the next card never waits on a
    /// helper that is ending.
    fn stop_running(&mut self) {
        if let Some(running) = self.running.take() {
            running.speech.stop();
        }
    }

    /// The disc on a speaking card: stop where it is. The line waits on it until play.
    pub fn pause(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.line.pause(id) {
            self.stop_running();
            cx.notify();
        }
    }

    /// The disc on a paused card, and Again on any: the card is said again from its start, next. What was
    /// speaking is stopped first, so two never overlap.
    pub fn again(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.line.again(id) {
            self.stop_running();
            self.dispatch(cx);
            cx.notify();
        }
    }

    /// A walk's mock speaker is told the line in hand is done, so a walk does not wait out its time.
    pub fn finish_now(&mut self, cx: &mut Context<Self>) {
        let Some(run) = self.running.as_ref().map(|running| running.run) else { return };
        let Some(id) = self.line.cards().iter().find(|card| card.run == run).map(|card| card.id) else { return };
        self.stop_running();
        self.line.ended(id, run);
        self.dispatch(cx);
        cx.notify();
    }
}
