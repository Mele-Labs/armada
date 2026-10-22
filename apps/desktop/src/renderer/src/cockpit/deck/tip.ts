// What a thing on the map says when it is hovered or focused: the card the callout draws. Each is built from what the field
// already holds (a drone, a bay, a main, a pull request, a plot, a pin), so the card and the thing speak of one fact. Ported from
// proto/cockpit-flight-deck. #2301.

import { ShieldCheck, ShieldX, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { GLYPH as DOING_GLYPH, WORD as DOING_WORD } from "./doing";
import { pillsOf } from "./field";
import type { DroneIn, FileAct, Hue, PrIn } from "./layout";
import { STAGE_SAID } from "./plays";
import { STAGE } from "./shared";
import type { BayState } from "./bays";
import type { BuildTip } from "./build";
import type { ClashView } from "@armada/components";
import type { Clash } from "./touches";
import type { Outpost } from "./outposts";

/** What colours the card's top edge and its glow. Quiet is gray: a thing with no state to speak of. */
export type TipHue = Hue | "quiet" | "good";
type Tone = "calm" | "ask" | "bad" | "good";

export type Tip = {
  /** The kind, in small caps: Session, Main, Bay. */
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
  build?: BuildTip;
  /** The drone's key, where it has a ring to ask for an update about. The pinned card draws the button. */
  ask?: string;
  /** A card of its own, drawn under the kind in place of the title and the state: a step's real node card. */
  node?: ReactNode;
};

/** What a drone's callout says beyond the drone itself: its state as the list draws it, the call it holds, what it last did and when, its last hour. */
export type DroneFacts = { state: { Glyph: LucideIcon; said: string; tone: "calm" | "ask" | "bad" }; call?: string; last?: { says: string; ago: string }; ticks: readonly number[]; bay?: { slot: number; Glyph: LucideIcon; word: string }; build?: BuildTip };

/** A bay as the map draws its pad. */
export type PadIn = { key: string; slot: number; bay: BayState | "held" | "kept"; word: string; repo: string; Glyph: LucideIcon; holder?: string; /** It is the one that held it last. */ holderWas?: boolean; branch?: string };
export type GoalIn = { root: string; said: string; red: boolean };

/** What a drone's card names in small caps: a Job's kind, or for a subagent the Job it works for. */
export const droneLabel = (drone: Pick<DroneIn, "job">): string => drone.job ?? "Job";

export const droneTip = (drone: DroneIn, facts: DroneFacts | undefined, fallback: LucideIcon): Tip => ({
  label: droneLabel(drone),
  title: drone.title,
  hue: drone.hue,
  state: facts?.state ?? { Glyph: fallback, said: drone.said, tone: drone.hue },
  ...(drone.doing === undefined ? {} : { doing: { Glyph: DOING_GLYPH[drone.doing], word: DOING_WORD[drone.doing], ...(drone.via === undefined ? {} : { via: drone.via }) } }),
  pills: pillsOf(drone),
  ...(facts?.bay === undefined ? {} : { bay: facts.bay }),
  ...(facts?.last === undefined ? {} : { last: facts.last }),
  ...(facts?.call === undefined ? {} : { call: facts.call }),
  ...(facts === undefined ? {} : { ticks: facts.ticks }),
  ...(facts?.build === undefined ? {} : { build: facts.build, ask: drone.key }),
});

export const padTip = (pad: PadIn): Tip => ({
  label: "Bay",
  title: `slot-${pad.slot}`,
  hue: pad.bay === "stranded" ? "ask" : pad.bay === "held" || pad.bay === "kept" ? "calm" : "quiet",
  state: { Glyph: pad.Glyph, said: pad.word, tone: pad.bay === "stranded" ? "ask" : "calm" },
  pills: [pad.repo, ...(pad.branch === undefined ? [] : [pad.branch])],
  ...(pad.holder === undefined ? {} : { lines: [`${pad.holderWas === true ? "Last holder, " : ""}Job: ${pad.holder}`] }),
});

/** What main says when it is hovered, and what a screen reader says of the flag. */
export const goalTip = (goal: GoalIn, repo: string): Tip => ({ label: "Main", title: `main in ${repo}`, hue: goal.red ? "bad" : "good", state: { Glyph: goal.red ? ShieldX : ShieldCheck, said: goal.red ? "Red" : "Green", tone: goal.red ? "bad" : "good" } });
export const goalSaid = (goal: GoalIn, repo: string) => `main in ${repo}: ${goal.red ? "red" : "green"}, objective`;

/** What a pull request says when it is hovered, and what a screen reader says of its crate. */
export const payloadTip = (pr: PrIn): Tip => ({ label: "Pull request", title: `#${pr.number}`, hue: pr.stage === "red" ? "bad" : "calm", state: { Glyph: STAGE[pr.stage].Glyph, said: STAGE[pr.stage].said, tone: pr.stage === "red" ? "bad" : "calm" } });
export const payloadSaid = (pr: PrIn) => `Pull request #${pr.number}, ${STAGE_SAID[pr.stage]}`;

/** What a service outpost says when it is hovered or focused: the service, and its recent calls, newest first. */
export const outpostTip = (outpost: Pick<Outpost, "name" | "lines">): Tip => ({ label: "Service", title: outpost.name, hue: "quiet", lines: outpost.lines });

/** What is being done to a file, as a tooltip names it. */
export const FILE_SAID: Record<FileAct | "rest", string> = { write: "Writing", edit: "Editing", remove: "Removing", rest: "File" };

export const NEEDS_YOU: readonly string[] = ["Needs you"];
/** What a plot under a write clash says: two drones where the map holds both, else that something else changed it too. */
export const clashLines = (clash: Pick<Clash, "keys">): readonly string[] => [clash.keys.length > 1 ? "Two drones are changing it" : "Changed elsewhere too"];

/** What a drone's clash mark says: the other party, and the paths that conflict. */
export const clashTip = (clash: ClashView): Tip => ({ label: "Write clash", title: clash.with.label, hue: "ask", lines: clash.paths });
