// The categories Settings is split into, what a search finds in each, and which one is open.
//
// **Which category is open is the window's, not the screen's.** The command palette opens Settings at
// a category from anywhere, before the screen is mounted, so the choice lives here and the screen
// reads it — `openSettingsAt` is the palette's way in, and `App.tsx` only says where Settings is.

import { useSyncExternalStore } from "react";

export type SettingsSectionId = "fleet" | "machine" | "phone" | "theme" | "layout" | "keyboard" | "guides";

/** One category: its name, and the words a person might search for to find what is in it. */
export type SettingsSection = {
  id: SettingsSectionId;
  label: string;
  /** Its row in the command palette, where a bare `Guides` would read as the Guides screen. */
  palette: string;
  /** One entry per setting the category holds: its label, then other words that should find it. */
  settings: readonly (readonly string[])[];
};

export const SETTINGS_SECTIONS: readonly SettingsSection[] = [
  {
    id: "fleet",
    label: "Fleet", palette: "Fleet settings",
    settings: [
      ["Drones at once", "concurrency", "parallel", "agents"],
      ["Memory to keep free", "ram", "admission"],
      ["Disk to keep free", "storage", "space", "gib"],
      ["Checks at once", "verification", "parallel"],
    ],
  },
  {
    id: "machine",
    label: "This machine", palette: "This machine's settings",
    settings: [
      ["Helm action authority", "redirect", "read-only", "acting"],
      ["Draft pull requests", "pr", "draft"],
    ],
  },
  { id: "phone", label: "Phone", palette: "Phone pairing", settings: [["Pair a phone", "mobile", "gateway", "device", "unpair"]] },
  { id: "theme", label: "Theme", palette: "Theme", settings: [["Theme", "colours", "colors", "dark", "light", "appearance", "mods"]] },
  { id: "layout", label: "Layout", palette: "Layout", settings: [["Layout", "tabs", "panels", "rail", "hide", "order", "reset"]] },
  // The keyboard's own matches are the acts, counted by `KeyboardSettings`; these find the category.
  { id: "keyboard", label: "Keyboard shortcuts", palette: "Keyboard shortcuts", settings: [["Keyboard shortcuts", "keys", "key bindings", "hotkeys", "keymap", "rebind", "controls"]] },
  { id: "guides", label: "Guides", palette: "Guide settings", settings: [["Open a guide the first time I meet a piece", "help", "tutorial", "onboarding"]] },
];

/** Whether one setting's words hold every word of a search. */
export function matches(words: readonly string[], query: string): boolean {
  const said = words.join(" ").toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter((one) => one !== "")
    .every((one) => said.includes(one));
}

/** How many of a category's settings a search finds. */
export function matchesIn(section: SettingsSection, query: string): number {
  return section.settings.filter((words) => matches([section.label, ...words], query)).length;
}

// ---- Which category is open -------------------------------------------------------------------

let open: SettingsSectionId = "fleet";
const listeners = new Set<() => void>();

/** Open Settings at a category, from the palette's row id: `settings:keyboard`, or `fleet_settings`. */
export function openSettingsAt(id: string, go: () => void): void {
  const wanted = id === "fleet_settings" ? "fleet" : id.replace(/^settings:/, "");
  const found = SETTINGS_SECTIONS.find((one) => one.id === wanted);
  if (found === undefined) return;
  setSettingsSection(found.id);
  go();
}

export function setSettingsSection(id: SettingsSectionId): void {
  if (open === id) return;
  open = id;
  listeners.forEach((on) => on());
}

/** The category Settings draws, and the way to choose another. */
export function useSettingsSection(): [SettingsSectionId, (id: SettingsSectionId) => void] {
  const now = useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    () => open,
    () => open,
  );
  return [now, setSettingsSection];
}

/**
 * The palette's Settings rows: one that opens Settings, and one per category. A category's own words
 * are its aliases, so `hotkeys` finds Keyboard shortcuts without the word ever being drawn.
 */
export const SETTINGS_PALETTE: readonly { id: string; label: string; aliases: readonly string[] }[] = SETTINGS_SECTIONS.map((one) => ({
  // `fleet_settings` is the id the palette has always carried for its first row.
  id: one.id === "fleet" ? "fleet_settings" : `settings:${one.id}`,
  label: one.palette,
  aliases: ["settings", "preferences", one.label, ...one.settings.flat()],
}));
