// What a thing on the map says when it is hovered or focused: the card the callout draws. Each is built from what the field
// already holds (a drone, a bay, a main, a pull request, a plot, a pin), so the card and the thing speak of one fact. Ported from
// proto/cockpit-flight-deck. #2301.

import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { GLYPH as DOING_GLYPH, WORD as DOING_WORD } from "./doing";
import { pillsOf } from "./field";
import type { DroneIn, Hue } from "./layout";
import type { BayState } from "./bays";
import type { Clash } from "./touches";

/** What colours the card's top edge and its glow. Quiet is gray: a thing with no state to speak of. */
export type TipHue = Hue | "quiet" | "good";
type Tone = "calm" | "ask" | "bad" | "good";

export type Tip = {
  /** The kind, in small caps: Job, Session, Main, Bay. */
  label: string;
  title: string;
  hue: TipHue;
  state?: { Glyph: LucideIcon; said: string; tone: Tone };
  /** What it is doing over its plot, as the tool events say: a glyph, the word, and what it ran or touched. */
  doing?: { Glyph: LucideIcon; word: string; path?: string; via?: string };
  /** The directories it touches, the biggest two. */
  pills?: readonly string[];
  /** The bay it holds. */
  bay?: { slot: number; Glyph: LucideIcon; word: string };
  /** What it last did, and how long ago. */
  last?: { says: string; ago: string };
  /** The call it holds, in one line. */
  call?: string;
  /** The last hour of its events, oldest first, one bar for each five minutes. */
  ticks?: readonly number[];
  lines?: readonly string[];
  /** A card of its own, drawn under the kind in place of the title and the state: a step's real node card. */
  node?: ReactNode;
};

/** What a drone's callout says beyond the drone itself: its state as the list draws it, the call it holds, what it last did and when, its last hour. */
export type DroneFacts = { state: { Glyph: LucideIcon; said: string; tone: "calm" | "ask" | "bad" }; call?: string; last?: { says: string; ago: string }; ticks: readonly number[]; bay?: { slot: number; Glyph: LucideIcon; word: string } };

/** A bay as the map draws its pad. */
export type PadIn = { key: string; slot: number; bay: BayState | "held" | "kept"; word: string; repo: string; Glyph: LucideIcon; holder?: string; holderKind?: "job" | "session"; /** It is the one that held it last. */ holderWas?: boolean; branch?: string };
export type GoalIn = { root: string; said: string; red: boolean };

export const droneTip = (drone: DroneIn, facts: DroneFacts | undefined, fallback: LucideIcon): Tip => ({
  label: drone.kind === "job" ? "Job" : "Session",
  title: drone.title,
  hue: drone.hue,
  state: facts?.state ?? { Glyph: fallback, said: drone.said, tone: drone.hue },
  ...(drone.doing === undefined ? {} : { doing: { Glyph: DOING_GLYPH[drone.doing], word: DOING_WORD[drone.doing], ...(drone.via === undefined ? {} : { via: drone.via }) } }),
  pills: pillsOf(drone),
  ...(facts?.bay === undefined ? {} : { bay: facts.bay }),
  ...(facts?.last === undefined ? {} : { last: facts.last }),
  ...(facts?.call === undefined ? {} : { call: facts.call }),
  ...(facts === undefined ? {} : { ticks: facts.ticks }),
});

export const padTip = (pad: PadIn): Tip => ({
  label: "Bay",
  title: `slot-${pad.slot}`,
  hue: pad.bay === "stranded" ? "ask" : pad.bay === "held" || pad.bay === "kept" ? "calm" : "quiet",
  state: { Glyph: pad.Glyph, said: pad.word, tone: pad.bay === "stranded" ? "ask" : "calm" },
  pills: [pad.repo, ...(pad.branch === undefined ? [] : [pad.branch])],
  ...(pad.holder === undefined ? {} : { lines: [`${pad.holderWas === true ? "Last holder, " : ""}${pad.holderKind === "session" ? "Session" : "Job"}: ${pad.holder}`] }),
});

export const NEEDS_YOU: readonly string[] = ["Needs you"];
/** What a plot under a write clash says: two drones where the map holds both, else that something else changed it too. */
export const clashLines = (clash: Pick<Clash, "keys">): readonly string[] => [clash.keys.length > 1 ? "Two drones are changing it" : "Changed elsewhere too"];
