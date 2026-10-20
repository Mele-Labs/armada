// The window's quiet sounds: a tick when the keyboard moves through a list, a click when a control is pressed. Nothing else makes one.
// Synthesised, no audio files. Several takes are here for the owner to hear; he keeps one and the rest are deleted.

export type Take = "tick" | "tap" | "knock" | "pebble" | "wood" | "glass";
type Kind = "move" | "press";

const TAKES: readonly Take[] = ["tick", "tap", "knock", "pebble", "wood", "glass"];
/** The mock picks a take with `?sounds=`, so a walk can offer all three. A real window has no query and plays the first. */
const asked = typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("sounds");
export const TAKE: Take = TAKES.find((one) => one === asked) ?? "tick";

/** Every sound is normalised to the same peak, here, so the takes differ in character and not in loudness: felt more than heard. */
const PEAK = 10 ** (-30 / 20);
/** A sound this soon after the last is dropped, so a held key does not machine-gun. */
const GAP_MS = 25;

type Voice = {
  ms: number;
  /** A filtered-noise burst, centred on `hz`, lasting `ms` of the sound where given: under a tone, that is its attack. */
  noise?: { hz: number; q: number; ms?: number; level?: number };
  /** A pitched pluck that falls from `from` to `to`, with an optional overtone at a multiple of it. */
  tone?: { from: number; to: number; wave: "sine" | "triangle"; overtone?: { ratio: number; level: number } };
};

const VOICES: Record<Take, Record<Kind, Voice>> = {
  tick: {
    move: { ms: 6, noise: { hz: 3600, q: 1.4 } },
    press: { ms: 10, noise: { hz: 2500, q: 1 } },
  },
  // Between tick and wood: a short pluck with a noise attack, so it is pitched like wood and crisp like tick.
  tap: {
    move: { ms: 14, noise: { hz: 3200, q: 1.2, ms: 3, level: 0.6 }, tone: { from: 1250, to: 1050, wave: "triangle" } },
    press: { ms: 18, noise: { hz: 2600, q: 1, ms: 3, level: 0.6 }, tone: { from: 1000, to: 820, wave: "triangle" } },
  },
  knock: {
    move: { ms: 20, noise: { hz: 2400, q: 1.2, ms: 2, level: 0.5 }, tone: { from: 980, to: 820, wave: "triangle" } },
    press: { ms: 26, noise: { hz: 2000, q: 1, ms: 2, level: 0.5 }, tone: { from: 780, to: 620, wave: "triangle" } },
  },
  pebble: {
    move: { ms: 12, noise: { hz: 4000, q: 1.4, ms: 2, level: 0.4 }, tone: { from: 1600, to: 1400, wave: "sine" } },
    press: { ms: 16, noise: { hz: 3200, q: 1.2, ms: 2, level: 0.4 }, tone: { from: 1300, to: 1100, wave: "sine" } },
  },
  wood: {
    move: { ms: 30, tone: { from: 840, to: 720, wave: "triangle" } },
    press: { ms: 38, tone: { from: 660, to: 520, wave: "triangle" } },
  },
  glass: {
    move: { ms: 38, tone: { from: 2800, to: 2800, wave: "sine", overtone: { ratio: 2.76, level: 0.18 } } },
    press: { ms: 46, tone: { from: 2200, to: 2150, wave: "sine", overtone: { ratio: 2.76, level: 0.22 } } },
  },
};

/** One voice as samples, peak at 1. Seeded, so a take sounds the same every time it plays. */
function render(voice: Voice, rate: number): Float32Array {
  const length = Math.max(2, Math.round((voice.ms / 1000) * rate));
  const out = new Float32Array(length);
  if (voice.noise !== undefined) {
    // RBJ band-pass, constant 0 dB peak, run over seeded white noise.
    const w = (2 * Math.PI * voice.noise.hz) / rate;
    const alpha = Math.sin(w) / (2 * voice.noise.q);
    const a0 = 1 + alpha;
    const b0 = alpha / a0;
    const a1 = (-2 * Math.cos(w)) / a0;
    const a2 = (1 - alpha) / a0;
    let seed = 0x2545f491;
    let x1 = 0;
    let x2 = 0;
    let y1 = 0;
    let y2 = 0;
    const level = voice.noise.level ?? 1;
    const burst = voice.noise.ms === undefined ? length : Math.min(length, Math.round((voice.noise.ms / 1000) * rate));
    for (let i = 0; i < burst; i += 1) {
      seed = (Math.imul(seed, 1664525) + 1013904223) | 0;
      const x = seed / 2147483648;
      const y = b0 * x - b0 * x2 - a1 * y1 - a2 * y2;
      x2 = x1;
      x1 = x;
      y2 = y1;
      y1 = y;
      // A burst shorter than the sound fades out across its own length, so it ends without a click.
      out[i] = y * level * (burst === length ? 1 : 1 - i / burst);
    }
  }
  if (voice.tone !== undefined) {
    const { from, to, wave, overtone } = voice.tone;
    let phase = 0;
    for (let i = 0; i < length; i += 1) {
      // The pitch falls exponentially across the sound: the small drop is what makes it a pluck and not a beep.
      const hz = from * (to / from) ** (i / length);
      phase += (2 * Math.PI * hz) / rate;
      const main = wave === "sine" ? Math.sin(phase) : (2 / Math.PI) * Math.asin(Math.sin(phase));
      out[i] = out[i]! + main + (overtone === undefined ? 0 : overtone.level * Math.sin(phase * overtone.ratio));
    }
  }
  // A half-millisecond rise so it does not click, then an exponential fall to a thousandth.
  const rise = Math.max(1, Math.round(0.0005 * rate));
  let peak = 0;
  for (let i = 0; i < length; i += 1) {
    out[i] = out[i]! * Math.min(1, i / rise) * Math.exp((-6.9 * i) / length);
    peak = Math.max(peak, Math.abs(out[i]!));
  }
  if (peak > 0) for (let i = 0; i < length; i += 1) out[i] = out[i]! / peak;
  out[length - 1] = 0;
  return out;
}

let context: AudioContext | null | undefined;
const buffers = new Map<Kind, AudioBuffer>();

/** Created on the first sound, which is always inside a press or a key, so the browser lets it start. */
function audio(): AudioContext | null {
  if (context === undefined) context = typeof AudioContext === "undefined" ? null : new AudioContext();
  return context;
}

let enabled = true;
let last = Number.NEGATIVE_INFINITY;

function play(kind: Kind): void {
  const now = performance.now();
  if (!enabled || now - last < GAP_MS) return;
  const ac = audio();
  if (ac === null) return;
  last = now;
  if (ac.state === "suspended") void ac.resume();
  let buffer = buffers.get(kind);
  if (buffer === undefined) {
    const samples = render(VOICES[TAKE][kind], ac.sampleRate);
    buffer = ac.createBuffer(1, samples.length, ac.sampleRate);
    buffer.copyToChannel(samples as Float32Array<ArrayBuffer>, 0);
    buffers.set(kind, buffer);
  }
  const source = ac.createBufferSource();
  const gain = ac.createGain();
  gain.gain.value = PEAK;
  source.buffer = buffer;
  source.connect(gain).connect(ac.destination);
  source.start();
}

/** `features.uiSounds`: on unless settings.json says false, and on until Fleet has answered. */
export function setUiSounds(on: boolean): void {
  enabled = on;
}

const PRESSABLE = 'button, [role="button"], [role="switch"], [role="tab"], input[type="checkbox"]';
/** What a list marks its selected row with. Several elements match at once (the open Place's tab, say), so the whole set is compared. */
const MARKS = '[aria-selected="true"], [aria-current]:not([aria-current="false"]), [data-selected], [data-cursor]';
const MOVES = new Set(["j", "k", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]);
const PLAIN_INPUTS = new Set(["checkbox", "radio", "button", "submit", "reset", "range", "image", "file", "color"]);

/** Somewhere a key is a letter or a caret move: j types there, and ← → move the caret. */
function typingIn(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  return target instanceof HTMLInputElement && !PLAIN_INPUTS.has(target.type);
}

type Seen = { focus: Element | null; marks: Element[] };

/** Where focus is and what is marked, read the same way before and after a key. A text box keeps focus, so only its marks count. */
function look(typing: boolean): Seen {
  return { focus: typing ? null : document.activeElement, marks: [...document.querySelectorAll(MARKS)] };
}

const same = (a: Seen, b: Seen) => a.focus === b.focus && a.marks.length === b.marks.length && a.marks.every((one, at) => one === b.marks[at]);

function onClick(event: MouseEvent): void {
  if (!enabled || !(event.target instanceof Element)) return;
  const control = event.target.closest(PRESSABLE);
  if (control === null || control.matches(':disabled, [aria-disabled="true"]')) return;
  play("press");
}

function onKey(event: KeyboardEvent): void {
  if (!enabled || !MOVES.has(event.key) || event.metaKey || event.ctrlKey || event.altKey) return;
  const typing = typingIn(event.target);
  // In a text box a letter is typed and ← → move the caret. ↑ ↓ may still move a list the box drives, a palette's results, and that shows in the marks.
  if (typing && event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
  const before = look(typing);
  // The key is detected by what it did, not by being pressed: at the end of a list it did nothing, and nothing sounds.
  requestAnimationFrame(() => {
    if (!same(before, look(typing))) play("move");
  });
}

/** One listener pair for the whole window, in the capture phase so a component that stops a key still sounds. Returns its removal. */
export function installUiSounds(): () => void {
  document.addEventListener("click", onClick, true);
  document.addEventListener("keydown", onKey, true);
  return () => {
    document.removeEventListener("click", onClick, true);
    document.removeEventListener("keydown", onKey, true);
  };
}
