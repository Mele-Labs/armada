// The two takes of the quick Dispatch panel play one script: only where the panel sits differs. Over `quick-dispatch`.

import { button, dialog, inside, place, region, role, tab, walk, type Step } from "./walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const strip = role("group", "Last dispatch");
const cockpit = region("Jobs and Sessions");
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);
const repository = (name: string) => inside(frame, role("radio", `Repository ${name}`));
const row = inside(frame, role("radiogroup", "Repository"));

const STEPS: readonly Step[] = [
  // (a) A Session, in one press from the Cockpit.
  { key: "n", on: cockpit, say: "N opens the Dispatch panel with the caret in the request" },
  { look: row, say: "Every set-up repository is a chip, each with its key" },
  { type: "Add a docs-site page for the retry settings", into: inPanel(request), say: "A request naming docs-site" },
  { look: inside(frame, role("img", "Repository docs-site")), say: "docs-site is read from the words and its chip fills" },
  { key: "Tab", on: inPanel(request), say: "Press Tab" },
  { look: inside(frame, tab("Session")), say: "Session, Job and Autopilot sit on the band, and Tab steps through them" },
  { key: "Alt+2", on: inPanel(request), say: "Press ⌥2" },
  { look: repository("armada"), say: "The second chip is chosen" },
  { key: "Enter", on: inPanel(request), say: "A Session starts on ↵" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },

  // (b) The Jobs surface's New Job opens the same panel.
  { press: place("Jobs"), say: "To Jobs" },
  { press: inside(region("Jobs"), button("New Job")), say: "New Job opens the same panel, on Job" },
  { look: row, say: "The chips, not a page with a dropdown" },

  // (c) Nothing chosen: the act gives the row the keys.
  { type: "Check the migration order", into: inPanel(request), say: "Words that name no repository" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch with none chosen" },
  { look: row, say: "The frame takes the call's hue and the row has the keys" },
  { key: "ArrowRight", on: repository("armada"), say: "→ moves along the row" },
  { press: repository("billing"), say: "↵ on a chip picks it and sends" },
  { look: strip, say: "A strip holds the Job's title where the panel was" },
];

/** The quick Dispatch walk, with the take's name. */
export const quickDispatchWalk = (): ReturnType<typeof walk> => walk("quick-dispatch", STEPS);

const deckPanel = role("region", "Dispatch controls");

/** What the two Cockpit panel takes share, after the sheet is up: read the repository, pick another, start a Session. */
const SHEET: readonly Step[] = [
  { look: row, say: "Every set-up repository is a chip, each with its key" },
  { look: inside(frame, role("img", "Repository docs-site")), say: "docs-site is read from the words and its chip fills" },
  { key: "Tab", on: inPanel(request), say: "Press Tab" },
  { look: inside(frame, tab("Session")), say: "Session, Job and Autopilot sit on the band, and Tab steps through them" },
  { key: "Alt+2", on: inPanel(request), say: "Press ⌥2" },
  { look: repository("armada"), say: "The second chip is chosen" },
  { key: "Enter", on: inPanel(request), say: "A Session starts on ↵" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },
];

const WORDS = "Add a docs-site page for the retry settings";

/** The Cockpit's Dispatch panel, between Flight control and Autopilot, as one tile that opens the centred sheet. */
export const quickDispatchDeckButton = (): ReturnType<typeof walk> =>
  walk("quick-dispatch", [
    { look: deckPanel, say: "A Dispatch panel stands between Flight control and Autopilot" },
    { hover: inside(deckPanel, button("Dispatch")), say: "The tile carries N" },
    { press: inside(deckPanel, button("Dispatch")), say: "Pressing it opens the sheet" },
    { type: WORDS, into: inPanel(request), say: "A request naming docs-site" },
    ...SHEET,
  ]);

/** The same panel as a one-line request box on the same draft: the first key typed grows the centred sheet. */
export const quickDispatchDeckBox = (): ReturnType<typeof walk> =>
  walk("quick-dispatch", [
    { look: deckPanel, say: "A Dispatch panel stands between Flight control and Autopilot" },
    { press: inside(deckPanel, role("textbox", "Request")), say: "A press puts the caret in its request" },
    { type: WORDS, into: inside(deckPanel, role("textbox", "Request")), say: "The first key typed grows the sheet, holding the words" },
    ...SHEET,
  ]);
