// The storyboard's seven shots as beats: a walk step, and what the mission-control overlay shows while it stands (`video-hud.tsx`).
// One list, so the walk `video-multitasking-boards` and the overlay cannot drift apart. Each `say` is the shot's storyboard line:
// its timecode, what happens and its voice over.

import { button, dialog, inside, place, region, role, text } from "./walk";
import type { Step, Target } from "./walk";

export type Beat = { clock: string; stamp?: string; step: Step };

const beat = (clock: string, stamp: string | undefined, step: Step): Beat => (stamp === undefined ? { clock, step } : { clock, stamp, step });

const projects = region("Projects", { exact: true });
const relay = region("Project Relay 2.4");
const jobs = region("Jobs", { exact: true });
const map = role("listbox", "Map");
const bays = region("Bays");
const queue = region("Merge queue");
const needs = region("Needs you");
const sheet = role("textbox", "Request");
const ask = role("article", "Waiting on you");

const row = (title: string): Target => inside(jobs, button(new RegExp(`^${title}`)));
const stage = (title: string): Target => region(new RegExp(`^Job: ${title}`));
const anyJob = region(/^Job: /);
const dispatch = (title: string): Target => button(`Dispatch ${title}`);
const holding = (title: string): Target => button(`Open ${title}`);

const [CHECKOUT, PAYMENTS, DARK, NOTES] = ["Fix the flaky checkout test", "Upgrade the payments SDK", "Dark mode for Settings", "Release notes for 2.4"] as const;
const walkWindow = dialog(`Bridge's window on ${DARK}`);

/** One launch: the Dispatch icon, the sheet it opens, the start, and the card that now names the Job. */
function launch(n: number, title: string, clock: string, first: string | undefined): Beat[] {
  const stamp = `FLIGHT ${n} · GO`;
  return [
    beat(clock, stamp, { hover: dispatch(title), say: first ?? `Flight ${n}: the Dispatch icon on the card for ${title}.` }),
    beat(clock, stamp, { press: dispatch(title), say: `The Dispatch sheet opens with ${title} already in the request.` }),
    beat(clock, stamp, { key: "Enter", on: sheet, say: `Enter starts Flight ${n}. The sheet closes and the Cockpit opens on its Job.` }),
    beat(clock, stamp, { press: place("Projects"), say: "Back on Projects." }),
    beat(clock, stamp, { look: holding(title), say: `The card now names its Job: ${title}.` }),
  ];
}

export const BEATS: readonly Beat[] = [
  // Shot 1, 0:00 to 0:05
  beat("T-00:05", undefined, { press: place("Projects"), say: "0:00 to 0:05, shot 1, countdown. A mono mission clock counts down in black. On zero the Projects tab cuts in. Voice over: You have four things to ship today." }),
  beat("T+00:00", undefined, { press: inside(projects, button("Relay 2.4", { exact: true })), say: "The project is Relay 2.4." }),
  beat("T+00:02", undefined, { look: relay, say: `Four open cards: ${CHECKOUT}, ${PAYMENTS}, ${DARK} and ${NOTES}. None has a Job yet.` }),

  // Shot 2, 0:05 to 0:15
  ...launch(1, CHECKOUT, "T+00:05", "0:05 to 0:15, shot 2, launch. The Dispatch icon on the first card. Voice over: Give each one to a Job."),
  ...launch(2, PAYMENTS, "T+00:07", undefined),
  ...launch(3, DARK, "T+00:09", undefined),
  beat("T+00:11", "FLIGHT 4 · GO", { hover: dispatch(NOTES), say: `Flight 4: the Dispatch icon on the card for ${NOTES}.` }),
  beat("T+00:11", "FLIGHT 4 · GO", { press: dispatch(NOTES), say: `The Dispatch sheet opens with ${NOTES} in the request.` }),
  beat("T+00:11", "FLIGHT 4 · GO", { type: `${NOTES} once @${PAYMENTS} ends`, into: sheet, say: `The request waits on ${PAYMENTS}: Flight 4 starts when Flight 2 ends.` }),
  beat("T+00:12", "FLIGHT 4 · GO", { press: inside(role("region", "Dispatch", { exact: true }), button("Start", { exact: true })), say: "Start. The Job is dispatched to wait and holds no bay." }),
  beat("T+00:13", undefined, { key: "Escape", on: anyJob, say: "A cut to the Cockpit." }),
  beat("T+00:13", undefined, { look: jobs, say: `Four Jobs in the list. ${NOTES} waits on ${PAYMENTS}.` }),
  beat("T+00:14", undefined, { look: bays, say: "Bays 1, 2 and 3 are held, one for each running Job. Bay 4 is free." }),

  // Shot 3, 0:15 to 0:25
  beat("T+00:15", undefined, { later: map, say: "0:15 to 0:25, shot 3, flight deck. The three running Jobs start to work and their drones lift off. Voice over: Each Job gets its own worktree. While they work, you can watch them fly." }),
  beat("T+00:17", undefined, { look: map, say: "The battlefield fills the stage. Relay is a territory with slabs for tests/checkout, payments, settings/ui and docs. Flights 1, 2 and 3 fly over their directories, a dotted line back to each bay. Main's flag is green. Flight 4 has no drone." }),

  // Shot 4, 0:25 to 0:37
  beat("T+00:25", undefined, { later: map, say: "0:25 to 0:37, shot 4, first call. Flight 2 stops on a question." }),
  beat("T+00:26", "FLIGHT 2 · REQUESTING DECISION", { look: map, say: "An amber pillar of light rises from Flight 2's drone, with a badge over it. Voice over: You hear from a Job only when it needs you." }),
  beat("T+00:29", "FLIGHT 2 · REQUESTING DECISION", { look: row(PAYMENTS), say: "The Flight 2 row turns yellow and pulses." }),
  beat("T+00:31", "FLIGHT 2 · REQUESTING DECISION", { key: "c", on: jobs, say: "c goes to the first row that needs you." }),
  beat("T+00:31", "FLIGHT 2 · REQUESTING DECISION", { key: "Enter", on: jobs, say: "Enter opens Flight 2 on the stage." }),
  beat("T+00:32", "FLIGHT 2 · REQUESTING DECISION", { look: inside(ask, text(/Migrate now, or pin v3/)), say: "The question: v4 drops the v1 webhooks. Migrate now, or pin v3?" }),
  beat("T+00:34", "FLIGHT 2 · REQUESTING DECISION", { press: inside(ask, role("radio", /^Migrate now/)), say: "The first option." }),
  beat("T+00:35", "FLIGHT 2 · REQUESTING DECISION", { press: inside(ask, button("Answer", { exact: true })), say: "Answer. The row stops pulsing." }),
  beat("T+00:36", "FLIGHT 2 · REQUESTING DECISION", { key: "Escape", on: stage(PAYMENTS), say: "Esc returns to the deck. Flights 1 and 3 kept working the whole time." }),

  // Shot 5, 0:37 to 0:52
  beat("T+00:37", undefined, { later: jobs, say: "0:37 to 0:52, shot 5, the walk. Flight 3 has a screen to show." }),
  beat("T+00:38", "FLIGHT 3 · VISUAL CHECK", { look: row(DARK), say: "The Flight 3 row pulses. Voice over: When it changes something you'd see, it shows you before it lands." }),
  beat("T+00:40", "FLIGHT 3 · VISUAL CHECK", { press: row(DARK), say: "The row opens Flight 3 on the stage." }),
  beat("T+00:41", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Needs you/)), say: "Needs you holds the page to look at." }),
  beat("T+00:42", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Open Dark mode/)), say: "A walk window opens on the new dark Settings screen." }),
  beat("T+00:45", "FLIGHT 3 · VISUAL CHECK", { look: walkWindow, say: "The window runs full frame. This is the one shot where the app is not the subject." }),
  beat("T+00:50", "FLIGHT 3 · VISUAL CHECK", { press: inside(walkWindow, button("Approve")), say: "Approve." }),
  beat("T+00:51", "FLIGHT 3 · VISUAL CHECK", { press: button("Close window"), say: "The window closes." }),
  beat("T+00:51", "FLIGHT 3 · VISUAL CHECK", { key: "Escape", on: stage(DARK), say: "Esc returns to the deck." }),

  // Shot 6, 0:52 to 1:00
  beat("T+00:52", undefined, { later: jobs, say: "0:52 to 1:00, shot 6, handoff. Flight 2's pull request opens." }),
  beat("T+00:56", "FLIGHT 4 · CLEARED", { look: row(NOTES), say: "The Flight 4 row stops saying after and starts on its own. Nobody presses anything. Voice over: Jobs can wait on each other, so you don't have to." }),
  beat("T+00:58", "FLIGHT 4 · CLEARED", { look: bays, say: "Flight 4 takes bay 4." }),

  // Shot 7, 1:00 to 1:20
  beat("T+01:00", undefined, { later: queue, say: "1:00 to 1:20, shot 7, landing. The pull requests slide into the merge queue in order. Voice over: Each one goes through the merge queue on a green main." }),
  beat("T+01:02", undefined, { look: queue, say: "#412, #413 and #414 are in the queue in order. Main is green." }),
  beat("T+01:03", undefined, { later: queue, say: "#412 reaches the front." }),
  beat("T+01:05", "FLIGHT 1 · TOUCHDOWN", { look: queue, say: "#412 merges and leaves the queue. Bay 1 is free. Main stays green." }),
  beat("T+01:06", "FLIGHT 1 · TOUCHDOWN", { later: queue, say: "#413 reaches the front. #415 enters the queue." }),
  beat("T+01:07", "FLIGHT 2 · TOUCHDOWN", { look: queue, say: "#413 merges. Bay 2 is free." }),
  beat("T+01:10", "FLIGHT 2 · TOUCHDOWN", { later: queue, say: "#414 reaches the front." }),
  beat("T+01:12", "FLIGHT 3 · TOUCHDOWN", { look: queue, say: "#414 merges. Bay 3 is free." }),
  beat("T+01:15", "FLIGHT 3 · TOUCHDOWN", { later: queue, say: "#415 reaches the front." }),
  beat("T+01:17", "FLIGHT 4 · TOUCHDOWN", { look: queue, say: "#415 merges. Bay 4 is free. Main is green." }),
  beat("T+01:17", undefined, { look: bays, say: "The deck at rest: every bay is free." }),
  beat("T+01:17", undefined, { look: jobs, say: "The four Jobs have ended. The mission clock stops at T+01:17." }),
];
