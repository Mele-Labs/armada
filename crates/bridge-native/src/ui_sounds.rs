//! The window's quiet sounds: a tick when the keyboard moves a list's selection, a click when a control is
//! pressed. Nothing else makes one. Synthesised, no audio files. The voice is Pebble, the take the owner
//! chose: a short sine pluck with a noise attack, the same samples Electron's `ui-sounds.ts` renders.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use gpui::{App, MouseButton, MouseDownEvent, Pixels, Point, Role, Window};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::NSSound;
use objc2_foundation::NSData;

use crate::labels::{Drawn, LabelBook};

/// Every sound is normalised to this peak: felt more than heard.
const PEAK_DB: f32 = -30.0;
/// A sound this soon after the last is dropped, so a held key does not machine-gun.
const GAP: Duration = Duration::from_millis(25);
const RATE: u32 = 48_000;

/// A sine falling from `from` to `to` over `ms`, under a band-passed noise burst at `hz` that is its attack.
struct Voice {
    ms: f32,
    from: f32,
    to: f32,
    hz: f32,
    q: f32,
    noise_ms: f32,
    level: f32,
}

/// The move is a touch higher and shorter than the press.
const MOVE: Voice = Voice { ms: 12.0, from: 1600.0, to: 1400.0, hz: 4000.0, q: 1.4, noise_ms: 2.0, level: 0.4 };
const PRESS: Voice = Voice { ms: 16.0, from: 1300.0, to: 1100.0, hz: 3200.0, q: 1.2, noise_ms: 2.0, level: 0.4 };

/// What a press sounds on: the roles Electron's `PRESSABLE` selector names.
const PRESSABLE: [Role; 4] = [Role::Button, Role::Switch, Role::Tab, Role::CheckBox];

/// The voice as samples, peak at 1. Seeded, so it sounds the same every time it plays.
fn render(voice: &Voice, rate: f32) -> Vec<f32> {
    let length = ((voice.ms / 1000.0 * rate).round() as usize).max(2);
    let mut out = vec![0.0f32; length];
    // RBJ band-pass, constant 0 dB peak, run over seeded white noise.
    let w = 2.0 * std::f32::consts::PI * voice.hz / rate;
    let alpha = w.sin() / (2.0 * voice.q);
    let a0 = 1.0 + alpha;
    let b0 = alpha / a0;
    let a1 = -2.0 * w.cos() / a0;
    let a2 = (1.0 - alpha) / a0;
    let mut seed: i32 = 0x2545f491;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let burst = ((voice.noise_ms / 1000.0 * rate).round() as usize).clamp(1, length);
    for (i, sample) in out.iter_mut().enumerate().take(burst) {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = seed as f32 / 2_147_483_648.0;
        let y = b0 * x - b0 * x2 - a1 * y1 - a2 * y2;
        x2 = x1;
        x1 = x;
        y2 = y1;
        y1 = y;
        // The burst fades across its own length, so it ends without a click.
        *sample = y * voice.level * (1.0 - i as f32 / burst as f32);
    }
    let mut phase = 0.0f32;
    for (i, sample) in out.iter_mut().enumerate() {
        // The small exponential pitch drop is what makes it a pluck and not a beep.
        phase += 2.0 * std::f32::consts::PI * voice.from * (voice.to / voice.from).powf(i as f32 / length as f32) / rate;
        *sample += phase.sin();
    }
    // A half-millisecond rise so it does not click, then an exponential fall to a thousandth.
    let rise = ((0.0005 * rate).round() as usize).max(1);
    let mut peak = 0.0f32;
    for (i, sample) in out.iter_mut().enumerate() {
        *sample *= (i as f32 / rise as f32).min(1.0) * (-6.9 * i as f32 / length as f32).exp();
        peak = peak.max(sample.abs());
    }
    if peak > 0.0 {
        out.iter_mut().for_each(|sample| *sample /= peak);
    }
    out[length - 1] = 0.0;
    out
}

/// Mono 16-bit PCM in a WAV file at `PEAK_DB`: what `NSSound` reads from memory.
fn wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let gain = 10f32.powf(PEAK_DB / 20.0);
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&((sample * gain * i16::MAX as f32).round() as i16).to_le_bytes());
    }
    out
}

#[derive(Clone, Copy)]
enum Kind {
    Move,
    Press,
}

thread_local! {
    /// Whether this process is the live window. Tests and walk checks that Jobs run in the background drive
    /// the same views, and their presses would click through the owner's speakers while nobody is there.
    static LIVE: Cell<bool> = const { Cell::new(false) };
    /// `features.uiSounds`: on unless Fleet's settings say false, and on until Fleet has answered.
    static ON: Cell<bool> = const { Cell::new(true) };
    static SOUNDS: RefCell<[Option<Retained<NSSound>>; 2]> = const { RefCell::new([None, None]) };
    static LAST: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Called once by the live window. Nothing else makes a sound.
pub fn allow() {
    LIVE.set(true);
}

/// Set from Fleet's settings as they arrive.
pub fn set_on(on: bool) {
    ON.set(on);
}

/// Whether a sound at `now` plays, and if so, `now` is the last one.
fn may_play(now: Instant) -> bool {
    if !LIVE.get() || !ON.get() || LAST.get().is_some_and(|last| now.duration_since(last) < GAP) {
        return false;
    }
    LAST.set(Some(now));
    true
}

/// Plays one sound, made the first time it plays and kept. A sound while the last still plays restarts it.
fn play(kind: Kind) {
    if !may_play(Instant::now()) {
        return;
    }
    SOUNDS.with_borrow_mut(|sounds| {
        let sound = &mut sounds[kind as usize];
        if sound.is_none() {
            let voice = match kind {
                Kind::Move => &MOVE,
                Kind::Press => &PRESS,
            };
            let bytes = wav(&render(voice, RATE as f32), RATE);
            *sound = NSSound::initWithData(NSSound::alloc(), &NSData::with_bytes(&bytes));
        }
        if let Some(sound) = sound {
            sound.stop();
            sound.play();
        }
    });
}

/// The keyboard moved a list's selection, a rail's or a grid's. Called by the list, only when the selection
/// is somewhere new: at the end of a list the key does nothing, and nothing sounds.
pub fn moved() {
    play(Kind::Move);
}

/// Whether a press at `at` lands on something pressable: the innermost labelled element painted under
/// it, or one it is drawn in, has a pressable role. Electron's `closest(PRESSABLE)`.
fn on_pressable(drawn: &[Drawn], at: Point<Pixels>) -> bool {
    let mut at = drawn.iter().enumerate().filter(|(_, one)| one.shown.contains(&at)).max_by_key(|(_, one)| one.painted).map(|(index, _)| index);
    while let Some(index) = at {
        if PRESSABLE.contains(&drawn[index].role) {
            return true;
        }
        at = drawn[index].parent;
    }
    false
}

/// The window root's capture listener: a left press on a pressable control clicks.
pub fn pressed(event: &MouseDownEvent, window: &mut Window, cx: &mut App) {
    if event.button != MouseButton::Left {
        return;
    }
    let Some(book) = cx.try_global::<LabelBook>() else { return };
    if on_pressable(book.drawn(window.window_handle().window_id()), event.position) {
        play(Kind::Press);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size, Bounds, SharedString};

    fn drawn(role: Role, origin: (f32, f32), extent: (f32, f32), parent: Option<usize>, painted: usize) -> Drawn {
        let bounds = Bounds::new(point(px(origin.0), px(origin.1)), size(px(extent.0), px(extent.1)));
        Drawn { role, name: SharedString::default(), bounds, shown: bounds, radius: None, parent, path: Vec::new(), painted }
    }

    #[test]
    fn a_press_clicks_on_a_button_and_what_it_holds_but_not_beside_it() {
        let page = vec![
            drawn(Role::Region, (0.0, 0.0), (400.0, 400.0), None, 1),
            drawn(Role::Button, (10.0, 10.0), (100.0, 30.0), Some(0), 2),
            drawn(Role::Label, (20.0, 15.0), (40.0, 20.0), Some(1), 3),
            drawn(Role::Row, (10.0, 100.0), (300.0, 30.0), Some(0), 4),
        ];
        assert!(on_pressable(&page, point(px(90.0), px(20.0))), "on the button");
        assert!(on_pressable(&page, point(px(30.0), px(20.0))), "on a label inside the button");
        assert!(!on_pressable(&page, point(px(50.0), px(110.0))), "a row is not a control");
        assert!(!on_pressable(&page, point(px(300.0), px(300.0))), "bare ground");
    }

    #[test]
    fn an_overlay_painted_later_takes_the_press() {
        let page = vec![drawn(Role::Button, (0.0, 0.0), (100.0, 100.0), None, 1), drawn(Role::Dialog, (0.0, 0.0), (200.0, 200.0), None, 2)];
        assert!(!on_pressable(&page, point(px(50.0), px(50.0))));
    }

    #[test]
    fn nothing_sounds_until_the_live_window_allows_it() {
        let now = Instant::now();
        assert!(!may_play(now), "a test or a walk check drives the views and must stay silent");
        allow();
        assert!(may_play(now + GAP), "the live window clicks");
        set_on(false);
        assert!(!may_play(now + GAP * 2), "features.uiSounds off");
    }

    #[test]
    fn the_press_is_a_short_pluck_at_minus_thirty_db() {
        let samples = render(&PRESS, RATE as f32);
        assert_eq!(samples.len(), 768, "16 ms at 48 kHz");
        assert!((samples.iter().fold(0.0f32, |most, one| most.max(one.abs())) - 1.0).abs() < 1e-6);
        assert_eq!(*samples.last().unwrap(), 0.0, "ends at rest so it does not click");
        let file = wav(&samples, RATE);
        assert_eq!(file.len(), 44 + samples.len() * 2);
        let loudest = file[44..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]]).unsigned_abs()).max().unwrap();
        assert!((1000..1100).contains(&loudest), "-30 dBFS is about 1036 of 32767, read {loudest}");
        assert_eq!(render(&MOVE, RATE as f32).len(), 576, "the move is 12 ms");
    }
}
