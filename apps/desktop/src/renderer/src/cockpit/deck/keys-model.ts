// The deck's panels and the sheet of its keys, read off the registry. **Nothing here spells a key the registry owns**: the key
// that goes to a panel is a slot of `deck_panel`, the tiles' digits are `deck_tile`'s row, and the sheet is drawn from the acts'
// own verbs and the keymap's spelling of them, so Settings → Keyboard moves a key and the sheet, the caps and the handlers
// follow. Two things are not the registry's: the panels' names, and the second letter of the page chord, which the
// `deck_page` row names in its notes. #2301.

import { actionOf, formatSlot, slotsOf } from "@armada/components";

export type PanelId = "list" | "activity" | "stage" | "bays" | "calls" | "queue" | "controls" | "autopilot";

export type Panel = {
  id: PanelId;
  name: string;
  /** Where the panel is drawn. */
  root: string;
  /** What the arrows move between inside it. */
  items?: string;
  /** Where focus lands when there is no better place. */
  home?: string;
  /** Left out of the round Tab and F6 go: its keys act while focus is in it. */
  aside?: true;
};

/** In the order Tab goes round them. */
export const PANELS: readonly Panel[] = [
  { id: "list", name: "Jobs and Sessions", root: ".deck-list", items: ".deck-row" },
  { id: "activity", name: "Activity", root: ".deck-activity", items: ".armada-tl__who", aside: true },
  { id: "stage", name: "the stage", root: ".deck-stage > .deck-pane" },
  { id: "bays", name: "Bays", root: ".deck-bays", items: ".deck-bay" },
  { id: "calls", name: "Incoming calls", root: ".deck-incoming", items: ".deck-hail" },
  { id: "queue", name: "Merge queue", root: ".deck-queue", items: "button" },
  { id: "controls", name: "Flight control", root: ".deck-controls", items: ".deck-tile" },
  { id: "autopilot", name: "Autopilot", root: ".deck-autopilot", items: "button", home: '[role="switch"]' },
];

/** The panels `deck_panel`'s slots go to, in the order the row lists its keys. */
export const PANEL_SLOTS: readonly PanelId[] = ["list", "bays", "calls", "queue", "controls", "autopilot"];

/** The panel a slot of `deck_panel` goes to. */
export const panelOfSlot = (slot: number): PanelId | undefined => PANEL_SLOTS[slot];

/** The key that goes to a panel, as this person has it, or `""` for a panel with no key (Activity and the stage are reached by Tab and by their own acts). */
export function panelKey(id: PanelId): string {
  const at = PANEL_SLOTS.indexOf(id);
  const slot = at < 0 ? undefined : slotsOf("deck_panel")[at];
  return slot === undefined ? "" : formatSlot(slot);
}

/** The second key of `g`: where it goes, by surface id the window resolves. */
export const PAGE_LETTERS = { c: "cockpit", j: "jobs", s: "sessions", y: "system" } as const;
export type PageLetter = keyof typeof PAGE_LETTERS;

export type SheetRow = { keys: readonly string[]; does: string };
export type SheetGroup = { head: string; rows: readonly SheetRow[] };

const caps = (id: string): string[] => slotsOf(id).map(formatSlot).filter((one) => one !== "");

/**
 * What the key sheet lists, read when it is drawn: every deck act that is built, with the keys it answers to now. An act that
 * is `not built` is not here, as it is not a key.
 */
export function keySheetOf(): readonly SheetGroup[] {
  const built = (id: string) => actionOf(id).unbuilt === null;
  const [down, up, downArrow, upArrow] = caps("move_focus");
  const [first] = caps("deck_page");
  return [
    {
      head: "Go",
      rows: [
        ...PANEL_SLOTS.map((id) => ({ keys: [panelKey(id)], does: PANELS.find((one) => one.id === id)!.name })),
        { keys: ["Tab", "⇧Tab"], does: "Next, previous panel" },
      ],
    },
    {
      head: "Within",
      rows: [
        { keys: [down, up, "←", "→", downArrow, upArrow].filter((one): one is string => one !== undefined), does: actionOf("move_focus").verb },
        { keys: caps("open_focused"), does: "Open" },
        { keys: caps("close"), does: actionOf("close").verb },
      ],
    },
    ...(built("deck_tile") ? [{ head: "Controls", rows: [{ keys: caps("deck_tile"), does: actionOf("deck_tile").verb }] }] : []),
    {
      head: "Calls",
      rows: [
        { keys: caps("call_recall"), does: "Open the next call" },
        { keys: caps("call_recall").map((one) => `⇧${one.toUpperCase()}`), does: "Open the previous call" },
      ],
    },
    ...(built("deck_timeline") ? [{ head: "Stage", rows: [{ keys: caps("deck_timeline"), does: actionOf("deck_timeline").verb }] }] : []),
    {
      head: "Pages",
      rows: [
        ...(built("deck_page") && first !== undefined ? (Object.entries(PAGE_LETTERS) as [PageLetter, string][]).map(([letter, place]) => ({ keys: [first, letter], does: place === "cockpit" ? "Cockpit" : place === "jobs" ? "Jobs" : place === "sessions" ? "Sessions" : "System" })) : []),
        ...(built("deck_tabs") ? [{ keys: caps("deck_tabs"), does: actionOf("deck_tabs").verb }] : []),
        { keys: caps("new_job"), does: actionOf("new_job").verb },
        { keys: caps("command_palette"), does: "Search" },
        { keys: caps("history"), does: actionOf("history").verb },
        { keys: caps("key_sheet"), does: actionOf("key_sheet").verb },
      ],
    },
  ];
}
