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
const needs = region("Needs you");
const sheet = role("textbox", "Request");
const ask = role("article", "Waiting on you");

const row = (title: string): Target => inside(jobs, button(new RegExp(`^${title}`)));
const stage = (title: string): Target => region(new RegExp(`^Job: ${title}`));
const anyJob = region(/^Job: /);

const [CHECKOUT, PAYMENTS, DARK, NOTES] = ["Fix the flaky checkout test", "Upgrade the payments SDK", "Dark mode for Settings", "Release notes for 2.4"] as const;
const walkWindow = dialog(`Bridge's window on ${DARK}`);

const REPOS = ["relay-web", "relay-api", "relay-app", "relay-docs"] as const;
const card = (title: string): Target => role("group", title);

export const BEATS: readonly Beat[] = [
  // Shot 1, 0:00 to 0:05
  beat("T-00:05", undefined, { press: place("Projects"), say: "0:00 to 0:05, shot 1, countdown. A mono mission clock counts down in black. On zero the Projects tab cuts in. Voice over: You have four things to ship today." }),
  beat("T+00:00", undefined, { press: inside(projects, button("Relay 2.4", { exact: true })), say: "The project is Relay 2.4." }),
  beat("T+00:02", undefined, { look: relay, say: `Four open cards, one in each repository: ${CHECKOUT}, ${PAYMENTS}, ${DARK} and ${NOTES}. None has a Job yet.` }),

  // Shot 2, 0:05 to 0:15
  beat("T+00:05", undefined, { key: " ", on: card(CHECKOUT), say: `0:05 to 0:15, shot 2, launch. Each card is picked with ⌘-click, starting with ${CHECKOUT} in ${REPOS[0]}. Voice over: Give each one to a Job.` }),
  beat("T+00:06", undefined, { key: " ", on: card(PAYMENTS), say: `${PAYMENTS}, in ${REPOS[1]}.` }),
  beat("T+00:06", undefined, { key: " ", on: card(DARK), say: `${DARK}, in ${REPOS[2]}.` }),
  beat("T+00:07", undefined, { key: " ", on: card(NOTES), say: `${NOTES}, in ${REPOS[3]}.` }),
  beat("T+00:07", undefined, { press: inside(relay, button("Dispatch", { exact: true })), say: "Dispatch, once, in the project's head. One sheet opens for each repository, the first now." }),
  beat("T+00:08", "FLIGHT 1 · GO", { look: sheet, say: `The sheet for ${REPOS[0]} holds ${CHECKOUT}.` }),
  beat("T+00:08", "FLIGHT 1 · GO", { key: "Enter", on: sheet, say: "Enter starts Flight 1. The sheet closes." }),
  beat("T+00:09", "FLIGHT 2 · GO", { look: sheet, say: `The next sheet is for ${REPOS[1]} and holds ${PAYMENTS}.` }),
  beat("T+00:09", "FLIGHT 2 · GO", { key: "Enter", on: sheet, say: "Enter starts Flight 2." }),
  beat("T+00:10", "FLIGHT 3 · GO", { look: sheet, say: `The next sheet is for ${REPOS[2]} and holds ${DARK}.` }),
  beat("T+00:10", "FLIGHT 3 · GO", { key: "Enter", on: sheet, say: "Enter starts Flight 3." }),
  beat("T+00:11", "FLIGHT 4 · GO", { look: sheet, say: `The last sheet is for ${REPOS[3]} and holds ${NOTES}.` }),
  beat("T+00:11", "FLIGHT 4 · GO", { type: `${NOTES} once @${PAYMENTS} ends`, into: sheet, say: `The request waits on ${PAYMENTS}, in another repository: Flight 4 starts when Flight 2 ends.` }),
  beat("T+00:12", "FLIGHT 4 · GO", { press: inside(role("region", "Dispatch", { exact: true }), button("Start", { exact: true })), say: "Start. The Job is dispatched to wait and holds no bay." }),
  beat("T+00:13", undefined, { press: inside(anyJob, button("Back", { exact: true })), say: "A cut to the Cockpit." }),
  beat("T+00:13", undefined, { look: jobs, say: `Four Jobs in the list, one in each repository. ${NOTES} waits on ${PAYMENTS}.` }),
  beat("T+00:14", undefined, { look: bays, say: `A bay is held in ${REPOS[0]}, ${REPOS[1]} and ${REPOS[2]}. ${REPOS[3]} holds none yet.` }),

  // Shot 3, 0:15 to 0:25
  beat("T+00:15", undefined, { later: map, say: "0:15 to 0:25, shot 3, flight deck. The three running Jobs start to work and their drones lift off. Voice over: Each Job gets its own worktree. While they work, you can watch them fly." }),
  beat("T+00:17", undefined, { look: map, say: "The battlefield fills the stage. Relay is four territories. relay-web has a slab for tests/checkout, relay-api for payments, relay-app for settings/ui and relay-docs for releases. Flights 1, 2 and 3 fly over theirs, a dotted line back to each bay. Each main's flag is green. relay-docs is quiet: Flight 4 has no drone." }),

  // Shot 4, 0:25 to 0:37
  beat("T+00:25", undefined, { later: map, say: "0:25 to 0:37, shot 4, first call. Flight 2 stops on a question." }),
  beat("T+00:26", "FLIGHT 2 · REQUESTING DECISION", { look: map, say: "An amber pillar of light rises from Flight 2's drone, with a badge over it. Voice over: You hear from a Job only when it needs you." }),
  beat("T+00:29", "FLIGHT 2 · REQUESTING DECISION", { look: row(PAYMENTS), say: "The Flight 2 row turns yellow and pulses." }),
  beat("T+00:31", "FLIGHT 2 · REQUESTING DECISION", { key: "c", on: row(CHECKOUT), say: "c goes to the first row that needs you." }),
  beat("T+00:31", "FLIGHT 2 · REQUESTING DECISION", { key: "Enter", on: row(PAYMENTS), say: "Enter opens Flight 2 on the stage." }),
  beat("T+00:32", "FLIGHT 2 · REQUESTING DECISION", { look: inside(ask, text(/Migrate now, or pin v3/)), say: "The question: v4 drops the v1 webhooks. Migrate now, or pin v3?" }),
  beat("T+00:34", "FLIGHT 2 · REQUESTING DECISION", { press: inside(ask, role("radio", /^Migrate now/)), say: "The first option." }),
  beat("T+00:35", "FLIGHT 2 · REQUESTING DECISION", { press: inside(ask, button("Answer", { exact: true })), say: "Answer. The row stops pulsing." }),
  beat("T+00:36", "FLIGHT 2 · REQUESTING DECISION", { press: inside(stage(PAYMENTS), button("Back", { exact: true })), say: "Back to the deck. Flights 1 and 3 kept working the whole time." }),

  // Shot 5, 0:37 to 0:52
  beat("T+00:37", undefined, { later: jobs, say: "0:37 to 0:52, shot 5, the walk. Flight 3 has a screen to show." }),
  beat("T+00:38", "FLIGHT 3 · VISUAL CHECK", { look: row(DARK), say: "The Flight 3 row pulses. Voice over: When it changes something you'd see, it shows you before it lands." }),
  beat("T+00:40", "FLIGHT 3 · VISUAL CHECK", { press: row(DARK), say: "The row opens Flight 3 on the stage." }),
  beat("T+00:41", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Needs you/)), say: "Needs you holds the page to look at." }),
  beat("T+00:42", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Open Dark mode/)), say: "A walk window opens on the new dark Settings screen." }),
  beat("T+00:45", "FLIGHT 3 · VISUAL CHECK", { look: walkWindow, say: "The window runs full frame. This is the one shot where the app is not the subject." }),
  beat("T+00:50", "FLIGHT 3 · VISUAL CHECK", { press: inside(walkWindow, button("Approve")), say: "Approve." }),
  beat("T+00:51", "FLIGHT 3 · VISUAL CHECK", { press: button("Close window"), say: "The window closes." }),
  beat("T+00:51", "FLIGHT 3 · VISUAL CHECK", { press: inside(stage(DARK), button("Back", { exact: true })), say: "Back to the deck." }),

  // Shot 6, 0:52 to 1:00
  beat("T+00:52", undefined, { later: jobs, say: "0:52 to 1:00, shot 6, handoff. Flight 2's pull request opens." }),
  beat("T+00:56", "FLIGHT 4 · CLEARED", { look: row(NOTES), say: "The Flight 4 row stops saying after and starts on its own. Nobody presses anything. Voice over: Jobs can wait on each other, so you don't have to." }),
  beat("T+00:58", "FLIGHT 4 · CLEARED", { look: bays, say: `Flight 4 takes a bay in ${REPOS[3]}.` }),

  // Shot 7, 1:00 to 1:20: touchdown on the battlefield, one drone and one pull request at a time
  beat("T+01:00", undefined, { later: map, say: "1:00 to 1:20, shot 7, touchdown. The pull requests ride their routes to their repositories' flags. Voice over: Each one goes through the merge queue on a green main." }),
  beat("T+01:02", undefined, { look: map, say: "#412, #413 and #414 are on their way, each to its own territory. Every main is green." }),
  beat("T+01:03", undefined, { later: map, say: "#412 reaches the front of relay-web's line." }),
  beat("T+01:05", "FLIGHT 1 · TOUCHDOWN", { look: map, say: "#412 merges in relay-web. Its bay is free. relay-web's main stays green." }),
  beat("T+01:06", undefined, { later: map, say: "#413 reaches the front of relay-api's line." }),
  beat("T+01:08", "FLIGHT 2 · TOUCHDOWN", { look: map, say: "#413 merges in relay-api. Its bay is free." }),
  beat("T+01:10", undefined, { later: map, say: "#414 reaches the front of relay-app's line." }),
  beat("T+01:12", "FLIGHT 3 · TOUCHDOWN", { look: map, say: "#414 merges in relay-app. Its bay is free." }),
  beat("T+01:14", undefined, { later: map, say: "#415 reaches the front of relay-docs' line." }),
  beat("T+01:17", "FLIGHT 4 · TOUCHDOWN", { look: map, say: "#415 merges in relay-docs. Its bay is free. Every main is green." }),

  // The bookend: the Projects tab, where the four cards that opened the film now ask whether they are done
  beat("T+01:17", undefined, { press: place("Projects"), say: "A cut back to the Projects tab." }),
  beat("T+01:17", undefined, { look: relay, say: "Relay 2.4. Each of the four cards now asks whether it is done, since its pull request merged." }),
  beat("T+01:17", undefined, { press: button("Show done", { exact: true }), say: "Show done, so a card stays where it is once it is answered." }),
  ...[CHECKOUT, PAYMENTS, DARK, NOTES].map((title) => beat("T+01:17", undefined, { press: inside(card(title), button("Mark done", { exact: true })), say: `Mark done: ${title}.` })),
  beat("T+01:17", undefined, { look: relay, say: "All four cards read done. The mission clock stops at T+01:17." }),
];
