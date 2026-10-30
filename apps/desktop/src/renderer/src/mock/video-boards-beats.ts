// The storyboard's eight shots as beats: a walk step, and what the mission-control overlay shows while it stands (`video-hud.tsx`).
// One list, so the walk `video-multitasking-boards` and the overlay cannot drift apart. Each `say` is the shot's storyboard line:
// its timecode, what happens and its caption. The end card, shot 9, is the edit's and has no step here.

import { NOTES_FIRST, NOTES_TITLE } from "./relay-notes-page";
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
const card = (title: string): Target => role("group", title);

const LEAD = "Ship Relay 2.4";
const [CHECKOUT, PAYMENTS, DARK, NOTES] = ["Fix the flaky checkout test", "Upgrade the payments SDK", "Dark mode for Settings", "Release notes for 2.4"] as const;
const [WEB, API, APP, DOCS] = ["relay-web", "relay-api", "relay-app", "relay-docs"] as const;
const walkWindow = dialog(`Bridge's window on ${DARK}`);

// Flight 2: the command in Needs you, and the terminal that opens under it.
const login = inside(needs, role("group", "payments login"));
const terminal = role("group", "Terminal", { exact: true });
const CODE = "4F7Q-2KD9";

// Flight 4: the Artifacts layer, and the page in the reading layer over the Job.
const artifacts = region("Artifacts", { exact: true });
const layer = region("Document", { exact: true });
const page = inside(layer, role("article", NOTES_TITLE, { exact: true }));
const contents = inside(layer, role("navigation", "Contents"));

// Flight 1: the Job browser in the side panel, and the stand-in staging site in it.
const browser = dialog("Job browser");
const browserPane = inside(browser, role("group", "Job browser", { exact: true }));
const shown = (name: string): Target => inside(browserPane, role("article", name));

export const BEATS: readonly Beat[] = [
  // Shot 1, 0:00 to 0:05
  beat("T-00:05", undefined, { press: place("Projects"), say: "0:00 to 0:05, shot 1, countdown. A mono mission clock counts down in black. On zero the Projects tab cuts in on the project. Caption: You have one release to ship today." }),
  beat("T+00:00", undefined, { press: inside(projects, button("Relay 2.4", { exact: true })), say: "The project is Relay 2.4." }),
  beat("T+00:02", undefined, { look: card(LEAD), say: `One card, ${LEAD}, with four subtasks under it. Nothing has a Job yet.` }),
  beat("T+00:03", undefined, { look: relay, say: `The subtasks are ${CHECKOUT} in ${WEB}, ${PAYMENTS} in ${API}, ${DARK} in ${APP} and ${NOTES} in ${DOCS}.` }),

  // Shot 2, 0:05 to 0:14
  beat("T+00:05", undefined, { press: inside(relay, button(`Dispatch ${LEAD}`, { exact: true })), say: `0:05 to 0:14, shot 2, launch. The cursor presses the Dispatch icon on ${LEAD}. Caption: Give it to one Job. It splits the work and dispatches the rest.` }),
  beat("T+00:06", undefined, { look: sheet, say: "The Dispatch sheet opens with the task and its four subtasks as lines." }),
  beat("T+00:07", "LEAD · GO", { key: "Enter", on: sheet, say: "Enter starts it. The lead Job opens on the stage in the Cockpit." }),
  beat("T+00:08", "LEAD · GO", { look: stage(LEAD), say: "One row in the Jobs list, the lead. It holds no bay." }),
  beat("T+00:10", "FLIGHT 1 · GO", { later: jobs, say: `The lead starts its first Job, in ${WEB}.` }),
  beat("T+00:10", "FLIGHT 1 · GO", { look: row(CHECKOUT), say: `${CHECKOUT} stands under the lead, joined to it by the linked Job line.` }),
  beat("T+00:11", "FLIGHT 2 · GO", { later: jobs, say: `The second, in ${API}.` }),
  beat("T+00:11", "FLIGHT 2 · GO", { look: row(PAYMENTS), say: `${PAYMENTS} stands under the lead.` }),
  beat("T+00:12", "FLIGHT 3 · GO", { later: jobs, say: `The third, in ${APP}.` }),
  beat("T+00:12", "FLIGHT 3 · GO", { look: row(DARK), say: `${DARK} stands under the lead.` }),
  beat("T+00:13", "FLIGHT 4 · GO", { later: jobs, say: `The fourth, in ${DOCS}.` }),
  beat("T+00:13", "FLIGHT 4 · GO", { look: row(NOTES), say: `${NOTES} stands under the lead.` }),
  beat("T+00:13", undefined, { press: inside(stage(LEAD), button("Back", { exact: true })), say: "Back to the deck." }),
  beat("T+00:14", undefined, { look: bays, say: `A bay is held in each repository: ${WEB}, ${API}, ${APP} and ${DOCS}.` }),

  // Shot 3, 0:14 to 0:24
  beat("T+00:15", undefined, { later: map, say: "0:14 to 0:24, shot 3, flight deck. The four Jobs start to work and their drones lift off. Caption: Four Jobs, four repositories, all at once. While they work, you can watch them fly." }),
  beat("T+00:17", undefined, { look: map, say: `The battlefield fills the stage. Relay is four territories. ${WEB} has a slab for tests/checkout, ${API} for payments, ${APP} for settings/ui and ${DOCS} for releases. Each drone flies over the slab it is changing, a dotted line back to its bay. Every main's flag is green.` }),

  // Shot 4, 0:24 to 0:34
  beat("T+00:24", undefined, { later: map, say: "0:24 to 0:34, shot 4, the terminal. Flight 2 needs a sign-in. Caption: You hear from a Job only when it needs you. This one needs your sign-in." }),
  beat("T+00:25", "FLIGHT 2 · REQUESTING SIGN-IN", { look: map, say: "An amber pillar of light rises from Flight 2's drone, with a badge over it." }),
  beat("T+00:26", "FLIGHT 2 · REQUESTING SIGN-IN", { look: row(PAYMENTS), say: "The Flight 2 row turns yellow and pulses." }),
  beat("T+00:27", "FLIGHT 2 · REQUESTING SIGN-IN", { key: "c", on: row(CHECKOUT), say: "c goes to the first row that needs you." }),
  beat("T+00:27", "FLIGHT 2 · REQUESTING SIGN-IN", { key: "Enter", on: row(PAYMENTS), say: "Enter opens Flight 2 on the stage." }),
  beat("T+00:28", "FLIGHT 2 · REQUESTING SIGN-IN", { press: inside(needs, button(/^Needs you/)), say: "Needs you holds the call." }),
  beat("T+00:28", "FLIGHT 2 · REQUESTING SIGN-IN", { look: login, say: "Its call is a command to run: payments login." }),
  beat("T+00:29", "FLIGHT 2 · REQUESTING SIGN-IN", { press: inside(login, button("Run", { exact: true })), say: "Run. A terminal docks under the Job." }),
  beat("T+00:30", "FLIGHT 2 · REQUESTING SIGN-IN", { look: inside(terminal, text(/Sign-in code:/)), say: "The payments CLI asks for a sign-in code." }),
  beat("T+00:31", "FLIGHT 2 · REQUESTING SIGN-IN", { type: `${CODE}\n`, into: role("textbox", "Terminal input"), say: `The code, ${CODE}, typed with the caret visible. Enter.` }),
  beat("T+00:32", "FLIGHT 2 · REQUESTING SIGN-IN", { look: inside(terminal, text(/Signed in to the payments sandbox/)), say: "The CLI says it is signed in." }),
  beat("T+00:33", "FLIGHT 2 · REQUESTING SIGN-IN", { press: inside(stage(PAYMENTS), button("Back", { exact: true })), say: "Back to the deck. The Flight 2 row has stopped pulsing." }),

  // Shot 5, 0:34 to 0:45
  beat("T+00:34", undefined, { later: jobs, say: "0:34 to 0:45, shot 5, the walk. Flight 3 has a screen to show. Caption: This one changed a screen, so it walks you through it." }),
  beat("T+00:35", "FLIGHT 3 · VISUAL CHECK", { look: row(DARK), say: "The Flight 3 row pulses." }),
  beat("T+00:36", "FLIGHT 3 · VISUAL CHECK", { press: row(DARK), say: "The row opens Flight 3 on the stage." }),
  beat("T+00:37", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Needs you/)), say: "Needs you holds the page to look at." }),
  beat("T+00:38", "FLIGHT 3 · VISUAL CHECK", { press: inside(needs, button(/^Open Dark mode/)), say: "A walk window opens on the new dark Settings screen." }),
  beat("T+00:40", "FLIGHT 3 · VISUAL CHECK", { look: walkWindow, say: "The window runs full frame and rings each changed part as it is stepped through. This is the one shot where the app is not the subject." }),
  beat("T+00:43", "FLIGHT 3 · VISUAL CHECK", { press: inside(walkWindow, button("Approve")), say: "Approve." }),
  beat("T+00:44", "FLIGHT 3 · VISUAL CHECK", { press: button("Close window"), say: "The window closes." }),
  beat("T+00:44", "FLIGHT 3 · VISUAL CHECK", { press: inside(stage(DARK), button("Back", { exact: true })), say: "Back to the deck." }),

  // Shot 6, 0:45 to 0:55
  beat("T+00:45", undefined, { later: jobs, say: "0:45 to 0:55, shot 6, the document. Flight 4 has written the release notes. Caption: This one wrote the release notes, and asks you to read them." }),
  beat("T+00:46", "FLIGHT 4 · REVIEW REQUESTED", { look: row(NOTES), say: "The Flight 4 row pulses." }),
  beat("T+00:46", "FLIGHT 4 · REVIEW REQUESTED", { press: row(NOTES), say: "The row opens Flight 4 on the stage." }),
  beat("T+00:47", "FLIGHT 4 · REVIEW REQUESTED", { press: button("Artifacts", { exact: true }), say: "Artifacts holds the page Flight 4 wrote." }),
  beat("T+00:48", "FLIGHT 4 · REVIEW REQUESTED", { press: inside(artifacts, button(NOTES_TITLE)), say: `The call is a document, ${NOTES_TITLE}. It opens in the reading layer, with its contents rail.` }),
  beat("T+00:49", "FLIGHT 4 · REVIEW REQUESTED", { press: inside(contents, button("Fixes")), say: "The cursor scrolls through the page by its contents." }),
  beat("T+00:50", "FLIGHT 4 · REVIEW REQUESTED", { press: inside(contents, button("What is new")), say: "Back to the first section." }),
  beat("T+00:51", "FLIGHT 4 · REVIEW REQUESTED", { select: NOTES_FIRST, in: page, say: "It selects one line." }),
  beat("T+00:51", "FLIGHT 4 · REVIEW REQUESTED", { key: "Shift+F10", on: page, say: "The menu opens at the selection." }),
  beat("T+00:52", "FLIGHT 4 · REVIEW REQUESTED", { press: role("menuitem", "Add note"), say: "Add note." }),
  beat("T+00:52", "FLIGHT 4 · REVIEW REQUESTED", { type: "Lead with dark mode\n", into: role("textbox", "Note"), say: "The note: Lead with dark mode." }),
  beat("T+00:53", "FLIGHT 4 · REVIEW REQUESTED", { key: "Meta+Enter", on: page, say: "Command Enter sends the note back to the Job." }),
  beat("T+00:54", "FLIGHT 4 · REVIEW REQUESTED", { key: "Escape", on: layer, say: "Esc closes the page. The note stands in the Job's thread." }),
  beat("T+00:54", "FLIGHT 4 · REVIEW REQUESTED", { press: inside(stage(NOTES), button("Back", { exact: true })), say: "Back to the deck." }),

  // Shot 7, 0:55 to 1:04
  beat("T+00:55", undefined, { later: jobs, say: "0:55 to 1:04, shot 7, the browser. Flight 4 has its note and carries on. Flight 1 has run staging. Caption: This one opens staging in Armada's browser, so you can watch checkout pass." }),
  beat("T+00:56", "FLIGHT 1 · LIVE CHECK", { look: row(CHECKOUT), say: "The Flight 1 row pulses, and the Flight 4 row has calmed." }),
  beat("T+00:57", "FLIGHT 1 · LIVE CHECK", { press: row(CHECKOUT), say: "The row opens Flight 1 on the stage." }),
  beat("T+00:57", "FLIGHT 1 · LIVE CHECK", { press: inside(region("Flight controls"), button("Browser", { exact: true })), say: "Browser, in Flight control." }),
  beat("T+00:58", "FLIGHT 1 · LIVE CHECK", { look: shown("Relay staging cart"), say: "Armada's browser opens beside the Job on the staging cart." }),
  beat("T+00:59", "FLIGHT 1 · LIVE CHECK", { later: browserPane, say: "The Job drives the page." }),
  beat("T+01:00", "FLIGHT 1 · LIVE CHECK", { look: shown("Relay staging card details"), say: "Card details." }),
  beat("T+01:01", "FLIGHT 1 · LIVE CHECK", { later: browserPane, say: "Pay." }),
  beat("T+01:01", "FLIGHT 1 · LIVE CHECK", { look: shown("Relay staging payment"), say: "The card is charged." }),
  beat("T+01:02", "FLIGHT 1 · LIVE CHECK", { later: browserPane, say: "The order confirmation." }),
  beat("T+01:02", "FLIGHT 1 · LIVE CHECK", { look: shown("Relay staging order confirmed"), say: "Order confirmed. The run that used to flake passes." }),
  beat("T+01:03", "FLIGHT 1 · LIVE CHECK", { press: inside(browser, button("Close")), say: "The browser closes." }),
  beat("T+01:03", "FLIGHT 1 · LIVE CHECK", { press: inside(ask, role("radio", /^Approve/)), say: "Approve." }),
  beat("T+01:04", "FLIGHT 1 · LIVE CHECK", { press: inside(ask, button("Answer", { exact: true })), say: "Answer. The row stops pulsing." }),
  beat("T+01:04", "FLIGHT 1 · LIVE CHECK", { press: inside(stage(CHECKOUT), button("Back", { exact: true })), say: "Back to the deck." }),

  // Shot 8, 1:04 to 1:20: touchdown on the battlefield, one drone and one pull request at a time
  beat("T+01:04", undefined, { later: map, say: "1:04 to 1:20, shot 8, landing. All four push and their pull requests ride their routes. Caption: Each one lands on a green main." }),
  beat("T+01:05", undefined, { look: map, say: "#412 runs its Checks in relay-web. #413, #414 and #415 are queued, each in its own repository. Every main is green." }),
  beat("T+01:06", undefined, { later: map, say: `#413 merges in ${API}.` }),
  beat("T+01:07", "FLIGHT 2 · TOUCHDOWN", { look: map, say: `relay-api · #413 merged. The flag flashes green and Flight 2's bay is free.` }),
  beat("T+01:09", undefined, { later: map, say: `#414 merges in ${APP}.` }),
  beat("T+01:10", "FLIGHT 3 · TOUCHDOWN", { look: map, say: `relay-app · #414 merged. Flight 3's bay is free.` }),
  beat("T+01:12", undefined, { later: map, say: `#415 merges in ${DOCS}.` }),
  beat("T+01:13", "FLIGHT 4 · TOUCHDOWN", { look: map, say: `relay-docs · #415 merged. Flight 4's bay is free.` }),
  beat("T+01:15", undefined, { later: map, say: `#412 merges in ${WEB}.` }),
  beat("T+01:16", "FLIGHT 1 · TOUCHDOWN", { look: map, say: `relay-web · #412 merged. Flight 1's bay is free. Every main is green.` }),
  beat("T+01:17", undefined, { later: map, say: "The lead Job ends." }),

  // The bookend: the Projects tab, where the card that opened the film and its subtasks now ask whether they are done
  beat("T+01:17", undefined, { press: place("Projects"), say: "A cut back to the Projects tab." }),
  beat("T+01:17", undefined, { look: relay, say: `${LEAD} and its four subtasks each ask whether they are done, since the Job that held them ended.` }),
  beat("T+01:17", undefined, { press: button("Show done", { exact: true }), say: "Show done, so a card stays where it is once it is answered." }),
  ...[CHECKOUT, PAYMENTS, DARK, NOTES, LEAD].map((title) => beat("T+01:17", undefined, { press: inside(card(title), button("Mark done", { exact: true })), say: `Mark done: ${title}.` })),
  beat("T+01:17", undefined, { look: relay, say: "The card and its four subtasks read done. The mission clock stops at T+01:17." }),
];
