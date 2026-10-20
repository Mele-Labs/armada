// What hovering or focusing a row of the deck's panels says: the inspect card, built from what the row already holds. A Session of the
// list and a bay each say their kind, name, state, facts and the key that opens them, in the one card every hover
// opens (`Inspect`). Nothing is read for a card, and none of these pins: the press on each is already an act.

import { GitPullRequest, KeyRound, SquareTerminal } from "lucide-react";
import { InspectCard, Stamp, type InspectFact, type InspectKey } from "@armada/components";

import { spanOf, type BayView } from "./bays";
import { toneOf } from "./order";
import type { Subject, Tone } from "./ship";

/** The press on a row of the deck opens it. */
export const OPENS: readonly InspectKey[] = [{ act: "open_focused", verb: "Open" }];

/** The status token a deck tone is drawn in: gray is calm, amber waits on him, red is broken. */
export const TONE_COLOR: Record<Tone, string> = { calm: "var(--fg-muted)", ask: "var(--status-awaiting-review)", bad: "var(--status-completed-failed)" };

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** A Session of the list. `calling` names what a Session of the list is calling him for, in the call's own words. */
export function SubjectCard({ one, calling }: { one: Subject; calling?: string }) {
  const kind = one.kind === "session" ? { Glyph: SquareTerminal, label: "Job" } : { Glyph: GitPullRequest, label: "Pull request" };
  const facts: InspectFact[] = [
    ...(calling === undefined ? [] : [{ label: "Calling", value: calling }]),
    ...(one.slot === undefined ? [] : [{ label: "Bay", value: `slot-${one.slot}`, mono: true }]),
    ...(one.pr === undefined ? [] : [{ label: "Pull request", value: `#${one.pr.number}`, mono: true }, { label: "Branch", value: one.pr.branch, mono: true }]),
    ...(one.at === undefined ? [] : [{ label: "Last", value: <Stamp first="age" at={one.at} />, mono: true }]),
  ];
  return <InspectCard kind={kind} where={one.repo} title={one.title} state={{ Glyph: one.mark.Glyph, said: one.mark.said, tone: TONE_COLOR[toneOf(one)] }} facts={facts} keys={OPENS} />;
}

/** A bay: its number and state, who holds it or held it last, and what the pool says about it. */
export function BayCard({ view, repo }: { view: BayView; repo: string }) {
  const holder = view.holder ?? view.last;
  const stranded = view.state === "stranded";
  const facts: InspectFact[] = [
    { label: "Repository", value: repo },
    ...(holder === undefined ? [] : [{ label: stranded ? "Last holder" : "Holder", value: `Job, ${holder.title ?? holder.id}` }]),
    ...(view.branch === undefined ? [] : [{ label: "Branch", value: view.branch, mono: true }]),
    ...(view.state === "leased" && view.heldFor !== undefined ? [{ label: "Held for", value: spanOf(view.heldFor), mono: true }] : []),
    ...((view.state === "free" || view.state === "closed") && view.freed !== undefined ? [{ label: "Given back", value: `${spanOf(view.freed)} ago`, mono: true }] : []),
    ...((view.dirty ?? 0) > 0 ? [{ label: "Uncommitted", value: plural(view.dirty!, "file", "files"), mono: true }] : []),
    ...((view.unpushed ?? 0) > 0 ? [{ label: "Not pushed", value: plural(view.unpushed!, "commit", "commits"), mono: true }] : []),
    ...(view.told === undefined ? [] : [{ label: "Holder told", value: <Stamp at={view.told} />, mono: true }]),
    ...(view.state === "free" || view.state === "closed" ? [{ label: "Checkout", value: view.warm ? "Warm" : "Cold" }] : []),
  ];
  const tone = stranded ? TONE_COLOR.ask : view.state === "leased" ? "var(--status-running)" : TONE_COLOR.calm;
  return <InspectCard kind={{ Glyph: KeyRound, label: "Bay" }} where={`slot-${view.slot.slot}`} title={holder?.title ?? `slot-${view.slot.slot}`} state={{ Glyph: view.Glyph, said: view.said ?? view.word, tone }} facts={facts} keys={OPENS} />;
}
