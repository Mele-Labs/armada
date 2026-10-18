// The Cockpit map with outposts at the far edge of the field, for the walk `field-outposts`: the ship at rest, and the services its Sessions
// have called outside the repository. GitHub, the issue tracker and the web have been called already; each `later` has a Session call
// one now (GitHub, then Figma, then the issue tracker), so a beam runs to its outpost and the outpost lights. Figma is called for the first
// time at the second, so its outpost appears then.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { CallService, TimelineEvent } from "@armada/protocol";

import { CLOCK_MS } from "../../cockpit/deck/field";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const ARMADA = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The ship's working Sessions, as its record names them. */
const GATE = { id: "p1", title: "Gate policy on every run" };
const CLOCK = { id: "r1", title: "Pin the store clock" };
const NOTES = { id: "w1", title: "Migration notes" };

const WORD: Record<string, string> = { forge: "GitHub", tracker: "the issue tracker", web: "the web", figma: "Figma" };

let count = 9100;
/** One call, as Fleet records it from the hooks that observe the tools: one entry each, dated when it was made. */
function call(at: string, who: { id: string; title: string }, service: CallService, line: string): TimelineEvent {
  count += 1;
  return { id: count, at, kind: "session.called", subject: { kind: "session", id: who.id }, title: who.title, tone: "run", says: `Called ${WORD[service] ?? service}`, detail: line, call: { service, name: WORD[service] ?? service, line }, manifest_id: ARMADA, live: true };
}

/** What was called before the walk begins, all of it too old to be a beam. */
const BEFORE = (): TimelineEvent[] => [
  call(ago(58), GATE, "forge", "gh pr view"),
  call(ago(41), CLOCK, "forge", "gh pr checks"),
  call(ago(33), NOTES, "web", "doc.rust-lang.org"),
  call(ago(27), NOTES, "tracker", "list_issues"),
  call(ago(15), CLOCK, "web", "docs.rs"),
];

/**
 * What is called at each moment: built when the moment comes, so it is dated then, five seconds back because the field reads the time in
 * five second steps and an entry dated after the step it is read at is not yet the present.
 */
const justNow = () => new Date(Date.now() - CLOCK_MS).toISOString();
const LATER: readonly (() => TimelineEvent[])[] = [
  () => [call(justNow(), GATE, "forge", "gh pr view")],
  () => [call(justNow(), CLOCK, "figma", "get_design_context")],
  () => [call(justNow(), NOTES, "tracker", "create_issue")],
];

function build(): Scenario {
  const draft = ship.draft!;
  return {
    ...ship,
    name: "field-outposts",
    says: "The Cockpit map with an outpost at the far edge for each service a Session has called: GitHub, the issue tracker and the web, with Figma once it is called",
    // The ship's own moments (a reply, main going red) are left out, so the Sessions stay at work.
    later: [],
    draft: {
      ...draft,
      events: [...(draft.events ?? []), ...BEFORE()],
      eventsLater: LATER,
      sessions: (control, more) => ({ ...draft.sessions!(control, more), later: () => undefined }),
    },
  };
}

export const fieldOutpostsShip: Scenario = build();
