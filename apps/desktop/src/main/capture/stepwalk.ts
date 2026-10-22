// A walk Bridge plays itself, from outside the page — `docs/practices/capture-window.md`, *A walk in
// the page*.
//
// A session that shows a window with `steps` gets the walk the mock plays in its own page, on any page:
// each step's target is found in an isolated world (`finder.ts`), ringed by the bar's view over the page,
// and acted on with real input events sent to the page's contents. **The page runs nothing of Bridge's
// and is handed nothing**: it hears a pointer and a keyboard, as it hears a person.
//
// The bar's walk row drives it as it drives a page's own `__armadaWalk`, through the same five acts.

import type { WalkStep, WalkTarget } from "@armada/protocol";
import type { CaptureRect, CaptureWalk } from "../../shared/capture-window";
import type { HeldKind, PageFinder } from "./finder";
import type { WalkAct } from "./walk";

type Mouse = Electron.MouseInputEvent;
type Key = Electron.KeyboardInputEvent;
/** What is sent to the page: input events, and the one thing they cannot spell, text outside the keyboard's range. */
export type Sent = Mouse | Key | { insert: string };

/** How long a step waits for its target, as `walk-find.ts`'s `PATIENCE_MS`, which the mock waits. */
export const PATIENCE_MS = 5_000;
/** How long autoplay holds each step before it goes on. */
export const DWELL_MS = 3_000;
/** How often the ringed box is read again: a panel slides, and a list scrolls under it. */
const FOLLOW_MS = 150;
/** The beat between the parts of a press, so the page draws a hover before the press and a press before the next step. */
const BEAT_MS = 40;
/** How long the bar's cursor takes to arrive, which a press waits for: the bar's `--duration-travel`. */
export const GLIDE_MS = 300;
/** How long a press stays in the state: the cursor and its rings have played out by then, and a bar drawn afresh does not play them again. */
export const PRESS_MS = 2_000;
/** Longest the report to the session is, and the words naming a target in the bar. */
const REPORT_MAX = 400;
const SAID_MAX = 240;

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** Whether a step acts on the page. A look, a hover and a later only show it. */
export function acts(step: WalkStep): boolean {
  return !("look" in step || "hover" in step || "later" in step);
}

/** The target a step aims at. */
export function targetOf(step: WalkStep): WalkTarget {
  if ("press" in step) return step.press;
  if ("look" in step) return step.look;
  if ("hover" in step) return step.hover;
  if ("later" in step) return step.later;
  if ("type" in step) return step.into;
  if ("key" in step) return step.on;
  if ("select" in step) return step.in;
  if ("paste" in step) return step.paste;
  return step.drag;
}

const quoted = (words: string) => `“${words}”`;

/** A target as a person would say it, the way `mock/walk.ts`'s `role()` and `text()` say theirs. */
export function saidOf(target: WalkTarget): string {
  const own =
    target.role === "text"
      ? `the words ${quoted(target.name ?? "")}`
      : `a ${target.role}${target.name === undefined ? "" : ` named ${quoted(target.name)}`}`;
  return target.within === undefined ? own : `${own} in ${saidOf(target.within)}`;
}

/** What the step does, said plainly for a report. */
function verb(step: WalkStep): string {
  if ("hover" in step) return "hover over";
  if ("key" in step) return "press a key on";
  if ("paste" in step) return "paste into";
  if ("select" in step) return "select words in";
  if ("press" in step) return "press";
  if ("drag" in step) return "drag";
  if ("type" in step) return "type into";
  return "look at";
}

const line = (words: string, max: number) => [...words.replace(/\s+/g, " ").trim()].slice(0, max).join("");

/** Why the walk holds on a step: what the session reads, naming the step and its target. */
export function heldAt(at: number, steps: readonly WalkStep[], why: "target" | "words"): string {
  const step = steps[at];
  if (step === undefined) return "";
  const place = `step ${at + 1} of ${steps.length}, ${quoted(line(step.say, 80))}`;
  const missing =
    why === "words" && "select" in step
      ? `the words ${quoted(line(step.select, 80))} are not in ${saidOf(step.in)}`
      : `nothing to ${verb(step)}: no ${saidOf(targetOf(step)).replace(/^an? /, "")} came within ${PATIENCE_MS / 1000} seconds`;
  return line(`${place}: ${missing}`, REPORT_MAX);
}

const centre = (box: CaptureRect) => ({ x: Math.round(box.x + box.width / 2), y: Math.round(box.y + box.height / 2) });

type Point = { x: number; y: number };

const move = (at: Point, held = false): Mouse => ({
  type: "mouseMove",
  x: at.x,
  y: at.y,
  ...(held ? { button: "left" as const, modifiers: ["leftbuttondown" as const] } : {}),
});
const down = (at: Point): Mouse => ({ type: "mouseDown", x: at.x, y: at.y, button: "left", clickCount: 1 });
const up = (at: Point): Mouse => ({ type: "mouseUp", x: at.x, y: at.y, button: "left", clickCount: 1 });

/** A press as a pointer makes one: arrive, down, up, at the box's middle. The page hears a click. */
export function pressEvents(box: CaptureRect): Mouse[] {
  const at = centre(box);
  return [move(at), down(at), up(at)];
}

/** The pointer arriving and staying, for a hover. */
export function hoverEvents(box: CaptureRect): Mouse[] {
  return [move(centre(box))];
}

/** The pointer leaving, which ends a hover. */
export function leaveEvents(box: CaptureRect): Mouse[] {
  const at = centre(box);
  return [{ type: "mouseLeave", x: at.x, y: at.y }];
}

/** Picked up by the middle and put down `by` further on, with moves between so a canvas hears a drag. */
export function dragEvents(box: CaptureRect, by: Point): Mouse[] {
  const from = centre(box);
  const at = (part: number): Point => ({ x: Math.round(from.x + by.x * part), y: Math.round(from.y + by.y * part) });
  return [move(from), down(from), ...[0.25, 0.5, 0.75, 1].map((part) => move(at(part), true)), up(at(1))];
}

/** A drag across words, from where they start to where they end: what a pointer leaves selected. */
export function selectEvents(from: Point, to: Point): Mouse[] {
  const middle = { x: Math.round((from.x + to.x) / 2), y: Math.round((from.y + to.y) / 2) };
  return [move(from), down(from), move(middle, true), move(to, true), up(to)];
}

/** The keys Electron spells otherwise than `KeyboardEvent.key` does. */
const NAMED: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  " ": "Space",
  Space: "Space",
  Escape: "Escape",
  Esc: "Escape",
  Enter: "Enter",
  Return: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
};

const MODIFIERS: Record<string, "alt" | "shift" | "meta" | "control"> = {
  Alt: "alt",
  Shift: "shift",
  Meta: "meta",
  Control: "control",
};

/** The symbols a shifted key makes, which take Shift down with them. */
const SHIFTED = new Set([..."~!@#$%^&*()_+{}|:\"<>?"]);

/** One printable ASCII character as the keyboard sends it: down, the character, up. */
function typed(character: string): Key[] {
  const shift = (character >= "A" && character <= "Z") || SHIFTED.has(character);
  const code = character === " " ? "Space" : character.toUpperCase();
  const modifiers = shift ? { modifiers: ["shift" as const] } : {};
  return [
    { type: "keyDown", keyCode: code, ...modifiers },
    { type: "char", keyCode: character === " " ? " " : character, ...modifiers },
    { type: "keyUp", keyCode: code, ...modifiers },
  ];
}

const enter = (): Key[] => [
  { type: "keyDown", keyCode: "Enter" },
  { type: "char", keyCode: "Enter" },
  { type: "keyUp", keyCode: "Enter" },
];

/**
 * Words as typing sends them. Printable ASCII goes as keys, so a page that listens for them hears
 * them; anything else is inserted as text, since the keyboard's table has no name for it. **A newline at
 * the end is Enter**, as the mock's `type` ends a line.
 */
export function typeEvents(words: string): Sent[] {
  const submit = words.endsWith("\n");
  const body = submit ? words.slice(0, -1) : words;
  const sent: Sent[] = [];
  for (const character of body) {
    if (character === "\n") sent.push(...enter());
    else if (character >= " " && character <= "~") sent.push(...typed(character));
    else sent.push({ insert: character });
  }
  return submit ? [...sent, ...enter()] : sent;
}

/** A key or a chord as the keyboard sends it: `n`, `Tab`, `Meta+n`. */
export function keyEvents(spec: string): Key[] {
  const parts = spec.split("+");
  const name = parts[parts.length - 1] === "" ? "+" : parts[parts.length - 1]!;
  const modifiers = parts.slice(0, -1).flatMap((one) => MODIFIERS[one] ?? []);
  const withModifiers = modifiers.length === 0 ? {} : { modifiers };
  const code = NAMED[name] ?? (name.length === 1 ? name.toUpperCase() : name);
  // A character reaches a field only when no command key is down; a chord is a shortcut and not text.
  const text = name.length === 1 && !modifiers.includes("meta") && !modifiers.includes("control");
  return [
    { type: "keyDown", keyCode: code, ...withModifiers },
    ...(text ? [{ type: "char" as const, keyCode: name, ...withModifiers }] : []),
    { type: "keyUp", keyCode: code, ...withModifiers },
  ];
}

/**
 * Whether a finished load is the one the steps begin on: the first, of the shown page. **Not a sign-in
 * address's load, and not a provider's page** the window passed through, which the steps were not written
 * for and which the finder is asked of no more than the shown origin.
 */
export function beginsOn(load: { begun: boolean; signingIn: boolean; onShownOrigin: boolean }): boolean {
  return !load.begun && !load.signingIn && load.onShownOrigin;
}

/** What the walk needs of the window around it. */
export type StepHost = {
  finder: PageFinder;
  /** Real input, to the page's contents. `{ insert }` is text typed outside the keyboard's range. */
  send: (sent: Sent) => void;
  /** The editing commands the keyboard shortcuts for them would run. */
  command: (name: "paste" | "selectAll") => void;
  /** Give the keyboard to the page for keys that must reach a field, and take it back for the bar's own. */
  keyboard: (page: boolean) => void;
  /** The pinned address loaded again, resolving once it has. */
  reload: () => Promise<void>;
  /** The walk's state changed: the bar is told. */
  changed: () => void;
  /** A step cannot be played: whoever showed the window is told, in a line naming it. */
  stuck: (why: string) => void;
  /** Whether the window is still on screen. */
  open: () => boolean;
  /**
   * Whether the page is off this machine, so autoplay stops before a step that acts and waits for Next.
   * Absent: it is not, and autoplay plays every step.
   */
  guarded?: () => boolean;
};

type Phase = "seeking" | "ready" | "stopped" | "done";

export class StepWalk {
  private readonly name: string;
  private readonly steps: readonly WalkStep[];
  private readonly host: StepHost;
  private at = 0;
  /** Steps before this one run straight through with no ring and no dwell: Back is a replay. */
  private from = 0;
  private phase: Phase = "seeking";
  private box: CaptureRect | null = null;
  private autoplaying = false;
  private folded = false;
  /** The page is on a provider the Session named, or has its popup open. The step waits and is not held. */
  private signing = false;
  /** Bumped by anything that moves the walk, so a look or a dwell that lands afterwards is let go of. */
  private round = 0;
  /** The beat that reads the ringed box again, and the dwell autoplay holds a step for. */
  private following: ReturnType<typeof setTimeout> | null = null;
  private dwelling: ReturnType<typeof setTimeout> | null = null;
  /** The steps the session has been told it cannot be played, so a step is reported once until it is found. */
  private readonly told = new Set<number>();
  private stoppedOn: "target" | "words" = "target";
  /** Autoplay stopped on this step because it acts on a page off this machine, and Next is what is waited on. */
  private waiting = false;
  /** The press just shown to the person, which the bar draws a cursor to. `n` makes each one a new thing to play. */
  private pressed: { x: number; y: number; n: number } | null = null;
  private presses = 0;
  private fading: ReturnType<typeof setTimeout> | null = null;

  constructor(name: string, steps: readonly WalkStep[], host: StepHost) {
    this.name = name;
    this.steps = steps;
    this.host = host;
  }

  /** What the bar draws. Bounded by construction: a name, a caption, small whole numbers and a box. */
  state(): CaptureWalk {
    const step = this.steps[Math.min(this.at, this.steps.length - 1)];
    return {
      name: line(this.name, 80),
      at: Math.min(this.at + 1, this.steps.length),
      of: this.steps.length,
      said: line(step?.say ?? "", SAID_MAX),
      autoplay: this.autoplaying,
      ready: this.phase === "ready",
      done: this.phase === "done",
      guide: {
        box: this.phase === "ready" ? this.box : null,
        folded: this.folded,
        ...(this.signing ? { signingIn: true } : {}),
        ...(this.phase === "ready" && this.waiting && !this.signing ? { waiting: true } : {}),
        ...(this.phase === "stopped" && !this.signing ? { stopped: line(this.stoppedWords(), SAID_MAX) } : {}),
        ...(this.pressed === null ? {} : { press: this.pressed }),
      },
    };
  }

  private stoppedWords(): string {
    const step = this.steps[this.at];
    if (step === undefined) return "";
    return this.stoppedOn === "words" && "select" in step ? `the words ${quoted(step.select)} in ${saidOf(step.in)}` : saidOf(targetOf(step));
  }

  /** Begin on the page as it stands: the first load. */
  begin(): void {
    void this.run(0);
  }

  /**
   * The page is, or is no longer, being signed in on: it is on a provider the Session named, or its
   * popup is open. **While it is, the step being looked for is waited on with its patience held**, so a
   * target that cannot be on a provider's page is not read as missing, and nothing is told to the
   * Session. Back on the shown origin the same step is looked for again, with the whole patience.
   */
  signIn(on: boolean): void {
    if (this.signing === on) return;
    this.signing = on;
    this.host.changed();
    // A step held before the page left is looked for again on its return.
    if (!on && this.phase === "stopped" && this.host.open()) void this.seek(++this.round);
  }

  /** The bar's five acts, and the one that shows or hides the ring. */
  act(act: WalkAct): void {
    if (act === "next") void this.next();
    else if (act === "back") void this.restart(Math.max(0, this.at - 1));
    else if (act === "replay") void this.restart(0);
    else this.autoplay(act === "autoplay_on");
  }

  /** Draw the ring and the caption over the page, or leave the page to take presses. */
  guide(shown: boolean): void {
    if (this.folded === !shown) return;
    this.folded = !shown;
    this.host.changed();
  }

  private autoplay(on: boolean): void {
    if (this.autoplaying === on) return;
    this.autoplaying = on;
    if (on && this.phase === "ready") this.dwell();
    else if (!on && this.dwelling !== null) {
      clearTimeout(this.dwelling);
      this.dwelling = null;
    }
    this.host.changed();
  }

  /** The window closed, or the walk is let go of. */
  stop(): void {
    this.round += 1;
    this.clear();
    void this.host.finder.release();
  }

  private clear(): void {
    if (this.following !== null) clearTimeout(this.following);
    if (this.dwelling !== null) clearTimeout(this.dwelling);
    this.following = null;
    this.dwelling = null;
  }

  /** Autoplay: go on after the dwell, unless the step acts on a page off this machine. */
  private dwell(): void {
    if (this.dwelling !== null) clearTimeout(this.dwelling);
    if (this.holdsAutoplay()) return;
    const round = this.round;
    this.dwelling = setTimeout(() => {
      if (round !== this.round) return;
      if (!this.holdsAutoplay()) void this.next();
    }, DWELL_MS);
  }

  /** Whether nothing but a press of Next runs this step: it acts, on a page off this machine. */
  private guards(step: WalkStep): boolean {
    return acts(step) && this.host.guarded?.() === true;
  }

  /**
   * Autoplay stops before a step that acts on a page off this machine, and goes off. Next runs the
   * step, and Autoplay pressed again goes on to the next one that acts.
   */
  private holdsAutoplay(): boolean {
    const step = this.steps[this.at];
    if (step === undefined || !this.guards(step)) return false;
    this.autoplaying = false;
    this.waiting = true;
    this.host.changed();
    return true;
  }

  /** The page again, then the walk from step 1 with the steps before `from` run straight through. */
  private async restart(from: number): Promise<void> {
    const round = ++this.round;
    this.clear();
    this.phase = "seeking";
    this.box = null;
    this.waiting = false;
    this.host.changed();
    await this.host.reload();
    if (round !== this.round || !this.host.open()) return;
    await this.run(from);
  }

  /** Seek step `from`'s target after running everything before it. */
  private async run(from: number): Promise<void> {
    this.from = from;
    this.at = 0;
    this.told.clear();
    await this.seek(++this.round);
  }

  /** Look for the current step's target, and ring it. */
  private async seek(round: number): Promise<void> {
    while (round === this.round && this.host.open()) {
      const at = this.at;
      const step = this.steps[at];
      if (step === undefined) {
        this.finish();
        return;
      }
      this.phase = "seeking";
      this.box = null;
      this.waiting = false;
      this.host.changed();
      const box = await this.arrive(targetOf(step), round);
      if (round !== this.round) return;
      if (box === null) {
        this.hold("target");
        return;
      }
      this.told.delete(at);
      if (at < this.from) {
        // Back and Replay run the steps before the target straight through, but not one that acts on a page off this machine.
        if (!this.guards(step)) {
          if (!(await this.perform(step, box, round))) return;
          this.at += 1;
          continue;
        }
        this.from = at;
        this.waiting = true;
      }
      // A hover is shown with what it opens already open.
      if ("hover" in step) for (const event of hoverEvents(box)) this.host.send(event);
      this.phase = "ready";
      this.box = box;
      this.host.changed();
      this.follow(round);
      if (this.autoplaying) this.dwell();
      return;
    }
  }

  /** The target once it has arrived, or `null` after the patience a step has. Asked in short turns, so a page that navigates is waited out. */
  private async arrive(target: WalkTarget, round: number): Promise<CaptureRect | null> {
    let until = Date.now() + PATIENCE_MS;
    for (;;) {
      if (round !== this.round || !this.host.open()) return null;
      // On a provider's page the target is not expected, and the time it takes a person to sign in is not the step's.
      if (this.signing) {
        until = Date.now() + PATIENCE_MS;
        await sleep(150);
        continue;
      }
      const left = until - Date.now();
      if (left <= 0) return null;
      const box = await this.host.finder.arrive(target, Math.min(left, 1000));
      if (box !== null || round !== this.round) return box;
      await sleep(150);
    }
  }

  /** Read the ringed box again on a beat. A target that left the page is looked for again. */
  private follow(round: number): void {
    if (this.following !== null) clearTimeout(this.following);
    const look = async (): Promise<void> => {
      if (round !== this.round || this.phase !== "ready" || !this.host.open()) return;
      const box = await this.host.finder.box();
      if (round !== this.round || this.phase !== "ready") return;
      if (box === null) {
        void this.seek(++this.round);
        return;
      }
      const was = this.box;
      if (was === null || was.x !== box.x || was.y !== box.y || was.width !== box.width || was.height !== box.height) {
        this.box = box;
        this.host.changed();
      }
      this.following = setTimeout(() => void look(), FOLLOW_MS);
    };
    this.following = setTimeout(() => void look(), FOLLOW_MS);
  }

  /** The step acted on, and the walk on to the next. */
  private async next(): Promise<void> {
    if (this.phase !== "ready") return;
    const step = this.steps[this.at];
    if (step === undefined) return;
    const round = ++this.round;
    this.clear();
    // Acting: a second press while it does is no second act.
    this.phase = "seeking";
    this.box = null;
    this.waiting = false;
    this.host.changed();
    // The box is read once more: what the press aims at is where the target is now.
    const box = (await this.host.finder.box()) ?? (await this.arrive(targetOf(step), round));
    if (round !== this.round) return;
    if (box === null) {
      this.hold("target");
      return;
    }
    if (!(await this.perform(step, box, round, true))) return;
    this.at += 1;
    await this.seek(round);
  }

  /** Done: past the last step, the ring gone and the page left where the walk ended. */
  private finish(): void {
    this.phase = "done";
    this.box = null;
    this.autoplaying = false;
    this.host.changed();
    void this.host.finder.release();
  }

  /** The walk holds on this step and the session is told which, once. */
  private hold(why: "target" | "words"): void {
    this.phase = "stopped";
    this.stoppedOn = why;
    this.box = null;
    this.autoplaying = false;
    this.host.changed();
    if (this.told.has(this.at)) return;
    this.told.add(this.at);
    this.host.stuck(heldAt(this.at, this.steps, why));
  }

  /** Do what the step does to its target. `false`: the walk stopped on it, or was moved. `shown`: the person is watching, so a press is shown first. */
  private async perform(step: WalkStep, box: CaptureRect, round: number, shown = false): Promise<boolean> {
    const send = (events: readonly Sent[]) => events.forEach((one) => this.host.send(one));
    if ("press" in step) {
      if (shown) {
        // The bar's cursor goes to the target before the page hears the press.
        this.pressed = { ...centre(box), n: (this.presses += 1) };
        this.host.changed();
        if (this.fading !== null) clearTimeout(this.fading);
        this.fading = setTimeout(() => {
          this.pressed = null;
          this.host.changed();
        }, PRESS_MS);
        await sleep(GLIDE_MS);
        if (round !== this.round) return false;
      }
      for (const event of pressEvents(box)) {
        this.host.send(event);
        await sleep(BEAT_MS);
      }
    } else if ("hover" in step) {
      send(leaveEvents(box));
    } else if ("drag" in step) {
      for (const event of dragEvents(box, step.by)) {
        this.host.send(event);
        await sleep(BEAT_MS / 2);
      }
    } else if ("select" in step) {
      const across = await this.host.finder.words(step.select);
      if (round !== this.round) return false;
      if (across === null) {
        this.hold("words");
        return false;
      }
      for (const event of selectEvents(across.from, across.to)) this.host.send(event);
    } else if ("type" in step || "key" in step || "paste" in step) {
      const kind = await this.focused();
      if (round !== this.round) return false;
      if ("type" in step) {
        // A field is typed over, not into: what is in it is selected first, as a person retyping it would.
        if (kind === "field") this.host.command("selectAll");
        for (const event of typeEvents(step.type)) {
          this.host.send(event);
          await sleep(BEAT_MS / 4);
        }
      } else if ("key" in step) {
        send(keyEvents(step.key));
      } else {
        this.host.command("paste");
      }
      this.host.keyboard(false);
    }
    // A look and a later do nothing. A beat, so the page has drawn what the act did before the next step looks.
    await sleep(BEAT_MS);
    return round === this.round;
  }

  /** The held element given the keyboard, and the page given it to hear keys with. */
  private async focused(): Promise<HeldKind | null> {
    const kind = await this.host.finder.focus();
    this.host.keyboard(true);
    return kind;
  }
}
