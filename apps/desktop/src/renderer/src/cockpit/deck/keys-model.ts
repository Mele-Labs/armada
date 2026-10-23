// The deck's panels and the sheet of its keys, read off the registry. **Nothing here spells a key the registry owns**: the key
// that goes to a panel is a slot of `deck_panel`, the tiles' digits are `deck_tile`'s row, and the sheet is drawn from the acts'
// own verbs and the keymap's spelling of them, so Settings → Keyboard moves a key and the sheet, the caps and the handlers
// follow. Two things are not the registry's: the panels' names, and the second letter of the page chord, which the
// `deck_page` row names in its notes. #2301.

import { actionOf, formatSlot, slotsOf } from "@armada/components";

import { PAUSE_KEY, SESSION_TILES, tileKey } from "./session-tiles";

export type PanelId = "list" | "activity" | "stage" | "bays" | "queue" | "controls" | "dispatch" | "autopilot";

/** The slot of `deck_panel` that goes to the first row of the list that needs him, which is no panel of its own. */
export type NeedsSlot = "needs";

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
  { id: "list", name: "Jobs", root: ".deck-list", items: ".deck-row" },
  { id: "activity", name: "Activity", root: ".deck-activity", items: ".armada-tl__dot", aside: true },
  { id: "stage", name: "the stage", root: ".deck-stage > .deck-pane" },
  { id: "bays", name: "Bays", root: ".deck-bays", items: ".deck-bay" },
  { id: "queue", name: "Merge queue", root: ".deck-queue", items: "button" },
  { id: "controls", name: "Flight control", root: ".deck-controls", items: ".deck-tile" },
  { id: "dispatch", name: "Dispatch", root: ".deck-dispatch", items: ".deck-tile" },
  { id: "autopilot", name: "Autopilot", root: ".deck-autopilot", items: "button", home: '[role="switch"]' },
];

/** The panels that are lists of items to open: where `o` opens the one under the cursor as Enter does. Not the stage, whose page has keys of its own, nor the controls. */
export const OPENS: readonly PanelId[] = ["list", "activity", "bays", "queue"];

/** The panels `deck_panel`'s slots go to, in the order the row lists its keys. */
export const PANEL_SLOTS: readonly (PanelId | NeedsSlot)[] = ["list", "bays", "needs", "queue", "controls", "autopilot", "dispatch"];

/** The panel a slot of `deck_panel` goes to. */
export const panelOfSlot = (slot: number): PanelId | NeedsSlot | undefined => PANEL_SLOTS[slot];

/** The key that goes to a panel, as this person has it, or `""` for a panel with no key (Activity and the stage are reached by Tab and by their own acts). */
export function panelKey(id: PanelId | NeedsSlot): string {
  const at = PANEL_SLOTS.indexOf(id);
  const slot = at < 0 ? undefined : slotsOf("deck_panel")[at];
  return slot === undefined ? "" : formatSlot(slot);
}

/** The second key of `g`: where it goes, by surface id the window resolves. */
export const PAGE_LETTERS = { c: "cockpit", s: "sessions", y: "system" } as const;
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
        ...PANEL_SLOTS.map((id) => ({ keys: [panelKey(id)], does: id === "needs" ? "Needs you" : PANELS.find((one) => one.id === id)!.name })),
        { keys: ["Tab", "⇧Tab"], does: "Next, previous panel" },
        ...(built("deck_search") ? [{ keys: caps("deck_search"), does: actionOf("deck_search").verb }] : []),
      ],
    },
    {
      head: "Within",
      rows: [
        { keys: [down, up, "←", "→", downArrow, upArrow].filter((one): one is string => one !== undefined), does: actionOf("move_focus").verb },
        { keys: [...caps("open_focused"), ...caps("open")], does: "Open" },
        { keys: caps("close"), does: actionOf("close").verb },
        ...(built("deck_reorder") ? [{ keys: caps("deck_reorder"), does: actionOf("deck_reorder").verb }] : []),
      ],
    },
    ...(built("deck_tile")
      ? [
          {
            head: "Controls",
            rows: [
              { keys: caps("deck_tile"), does: actionOf("deck_tile").verb },
              // With a Session open the same keys press its tiles instead, ⌥⇧1 to ⌥⇧9 and ⌥⇧0; the last tile has no key.
              ...SESSION_TILES.flatMap((tile, at) => {
                const key = tileKey(at);
                return key === undefined ? [] : [{ keys: [key], does: `Job open: ${tile.label}` }];
              }),
              { keys: [PAUSE_KEY], does: "Pause the Job, or Resume it while paused" },
            ],
          },
        ]
      : []),
    {
      head: "Calls",
      rows: [
        { keys: caps("call_recall"), does: "Open the next call" },
        { keys: caps("call_recall").map((one) => `⇧${one.toUpperCase()}`), does: "Open the previous call" },
      ],
    },
    {
      head: "Stage",
      rows: [
        { keys: caps("dashboard_view"), does: "Show the map" },
        ...(built("deck_timeline") ? [{ keys: caps("deck_timeline"), does: actionOf("deck_timeline").verb }] : []),
        ...(built("deck_field") ? [{ keys: caps("deck_field"), does: actionOf("deck_field").verb }] : []),
      ],
    },
    // The map's own keys and a replay's are a mode's, not acts of the registry: they listen only while the map or a replay is up.
    {
      head: "Map",
      rows: [
        { keys: ["←", "→", "↑", "↓"], does: "Pan" },
        { keys: ["+", "-"], does: "Zoom" },
        { keys: ["0"], does: "The whole field" },
        { keys: ["Home"], does: "Locate the selection" },
      ],
    },
    {
      head: "Replay",
      rows: [
        { keys: ["Space"], does: "Play or pause" },
        { keys: ["←", "→"], does: "Step one event" },
        { keys: ["Home"], does: "Start over" },
        { keys: [...caps("close"), "⇧L"], does: "Back to Live" },
      ],
    },
    {
      head: "Pages",
      rows: [
        ...(built("deck_page") && first !== undefined ? (Object.entries(PAGE_LETTERS) as [PageLetter, string][]).map(([letter, place]) => ({ keys: [first, letter], does: place === "cockpit" ? "Cockpit" : place === "sessions" ? "Jobs" : "System" })) : []),
        ...(built("deck_tabs") ? [{ keys: caps("deck_tabs"), does: actionOf("deck_tabs").verb }] : []),
        ...(built("open_settings") ? [{ keys: caps("open_settings"), does: actionOf("open_settings").verb }] : []),
        { keys: caps("new_job"), does: actionOf("new_job").verb },
        { keys: caps("command_palette"), does: "Search" },
        { keys: caps("history"), does: actionOf("history").verb },
        { keys: caps("key_sheet"), does: actionOf("key_sheet").verb },
      ],
    },
  ];
}
