// The categories Settings is split into, what a search finds in each, and which one is open.
//
// **Which category is open is the window's, not the screen's.** The command palette opens Settings at
// a category from anywhere, before the screen is mounted, so the choice lives here and the screen
// reads it — `openSettingsAt` is the palette's way in, and `App.tsx` only says where Settings is.

import { useSyncExternalStore } from "react";
import type { Setting, SettingsList } from "@armada/protocol";

/**
 * A category's id. **A string, not a closed set**: the settings.json sections are Fleet's, read off
 * `get_settings`, and one Fleet adds is a category here without a change to this file.
 */
export type SettingsSectionId = string;

/** One category: its name, and the words a person might search for to find what is in it. */
export type SettingsSection = {
  id: SettingsSectionId;
  label: string;
  /** Its row in the command palette, where a bare `Guides` would read as the Guides screen. */
  palette: string;
  /** One entry per setting the category holds: its label, then other words that should find it. */
  settings: readonly (readonly string[])[];
  /** Drawn from settings.json's schema rather than by a pane of its own. */
  schema?: true;
};

/** A settings.json section, by the id its name makes: `Limits` is `limits`. */
export function sectionId(section: string): SettingsSectionId {
  return section.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

/**
 * The settings.json sections Fleet ships, so the palette can open one before Fleet has answered.
 * Their words are what a palette search finds them by; once the list is read, a search reads every
 * setting's own title, key and description instead.
 */
const SCHEMA_SECTIONS: readonly SettingsSection[] = [
  { id: "limits", label: "Limits", palette: "Fleet limits", schema: true, settings: [["Drones at once", "concurrency", "parallel", "memory", "disk", "checks at once", "cost cap", "turn cap", "ports"]] },
  { id: "timeouts", label: "Timeouts", palette: "Timeouts", schema: true, settings: [["Timeouts", "time limit", "seconds", "wait", "reconnect"]] },
  { id: "retention", label: "Retention", palette: "Retention", schema: true, settings: [["Retention", "keep", "days", "history", "clean up"]] },
  { id: "harness", label: "Harness", palette: "Harness", schema: true, settings: [["Harness", "agent", "program", "binary", "path"]] },
  { id: "model", label: "Model", palette: "Models", schema: true, settings: [["Models", "model", "roster", "opus", "sonnet", "haiku"]] },
  { id: "effort", label: "Effort", palette: "Effort", schema: true, settings: [["Effort", "reasoning", "low", "medium", "high"]] },
  { id: "prompts", label: "Prompts", palette: "Drone prompts", schema: true, settings: [["Prompts", "brief", "baseline", "instructions"]] },
  { id: "features", label: "Features", palette: "Features", schema: true, settings: [["Features", "switch", "helm", "draft pull requests"]] },
  { id: "editor", label: "Editor", palette: "Editor", schema: true, settings: [["Editor", "open", "code", "vscode", "visual"]] },
  { id: "terminal", label: "Terminal", palette: "Terminal", schema: true, settings: [["Terminal", "iterm", "ghostty", "warp", "wezterm", "shell"]] },
];

/** Bridge's own panes: their values live in settings.json too, and each is drawn by its own pane. */
const PANES: readonly SettingsSection[] = [
  { id: "phone", label: "Phone", palette: "Phone pairing", settings: [["Pair a phone", "mobile", "gateway", "device", "unpair"]] },
  { id: "theme", label: "Theme", palette: "Theme", settings: [["Theme", "colours", "colors", "dark", "light", "appearance", "mods"]] },
  { id: "layout", label: "Layout", palette: "Layout", settings: [["Layout", "tabs", "panels", "rail", "hide", "order", "reset"]] },
  // The keyboard's own matches are the acts, counted by `KeyboardSettings`; these find the category.
  { id: "keyboard", label: "Keyboard shortcuts", palette: "Keyboard shortcuts", settings: [["Keyboard shortcuts", "keys", "key bindings", "hotkeys", "keymap", "rebind", "controls"]] },
  { id: "guides", label: "Guides", palette: "Guide settings", settings: [["Open a guide the first time I meet a piece", "help", "tutorial", "onboarding"]] },
];

/** Every category the palette knows, whether or not Fleet has answered. */
export const SETTINGS_SECTIONS: readonly SettingsSection[] = [...SCHEMA_SECTIONS, ...PANES];

/** A setting the generic form draws. A Json value is Bridge's own, drawn by its pane. */
export function drawn(setting: Setting): boolean {
  return setting.kind.kind !== "json";
}

/**
 * The categories Settings offers for a reading of settings.json: each schema section the list holds,
 * in the order it first names them, then Bridge's panes. **Before Fleet answers, every shipped
 * schema section is offered**, and each says it has not been read.
 */
export function sectionsFor(list: SettingsList | null): readonly SettingsSection[] {
  if (list === null) return SETTINGS_SECTIONS;
  const order: string[] = [];
  const names = new Map<string, string>();
  for (const one of list.settings.filter(drawn)) {
    const id = sectionId(one.section);
    if (!names.has(id)) {
      order.push(id);
      names.set(id, one.section);
    }
  }
  const schema = order.map((id): SettingsSection => {
    const shipped = SCHEMA_SECTIONS.find((one) => one.id === id);
    return shipped ?? { id, label: names.get(id) ?? id, palette: names.get(id) ?? id, settings: [], schema: true };
  });
  return [...schema, ...PANES];
}

/** The settings in one schema section, as the generic form draws them. */
export function settingsIn(list: SettingsList, id: SettingsSectionId): Setting[] {
  return list.settings.filter((one) => drawn(one) && sectionId(one.section) === id);
}

/** One entry of Settings' index: a group of sections, drawn together on one page. */
export type SettingsGroup = { id: string; label: string; sections: readonly SettingsSection[] };

/**
 * The groups Bridge ships, in the index's order, and the sections each holds. A setting's own `group`
 * wins; this places the panes, and every section before Fleet has answered.
 */
const SHIPPED_GROUPS: readonly { id: string; label: string; sections: readonly string[] }[] = [
  { id: "agents", label: "Agents", sections: ["harness", "model", "effort"] },
  { id: "prompts", label: "Prompts", sections: ["prompts"] },
  { id: "fleet", label: "Fleet", sections: ["limits", "timeouts", "retention"] },
  { id: "features", label: "Features", sections: ["features"] },
  { id: "tools", label: "Tools", sections: ["editor", "terminal"] },
  { id: "appearance", label: "Appearance", sections: ["theme", "layout", "keyboard"] },
  { id: "phone", label: "Phone", sections: ["phone"] },
  { id: "guides", label: "Guides", sections: ["guides"] },
];

/** The shipped group a section is under, or `undefined` for one Bridge does not know. */
function shippedGroupOf(section: SettingsSectionId): string | undefined {
  return SHIPPED_GROUPS.find((one) => one.sections.includes(section))?.id;
}

/**
 * The index for a reading of settings.json: the shipped groups in their order, each holding the
 * sections this reading has, then any group Fleet names that Bridge does not ship. A group with no
 * section in it is not offered.
 */
export function groupsFor(list: SettingsList | null): readonly SettingsGroup[] {
  const named = new Map<SettingsSectionId, { id: string; label: string }>();
  for (const one of list?.settings.filter(drawn) ?? []) {
    const section = sectionId(one.section);
    if (named.has(section) || one.group === "") continue;
    named.set(section, { id: sectionId(one.group), label: one.group });
  }
  const groups: { id: string; label: string; sections: SettingsSection[] }[] = SHIPPED_GROUPS.map((one) => ({ id: one.id, label: one.label, sections: [] }));
  for (const section of sectionsFor(list)) {
    const wire = named.get(section.id);
    const id = wire?.id ?? shippedGroupOf(section.id) ?? section.id;
    let group = groups.find((one) => one.id === id);
    if (group === undefined) {
      group = { id, label: wire?.label ?? section.label, sections: [] };
      groups.push(group);
    }
    group.sections.push(section);
  }
  return groups.filter((one) => one.sections.length > 0);
}

/** The words a search reads for one setting: its title, its key, its description and its section. */
export function wordsOf(setting: Setting): string[] {
  return [setting.title, setting.key, setting.description, setting.section];
}

/** Whether one setting's words hold every word of a search. */
export function matches(words: readonly string[], query: string): boolean {
  const said = words.join(" ").toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter((one) => one !== "")
    .every((one) => said.includes(one));
}

/**
 * How many of a category's settings a search finds. A schema section, once settings.json is read,
 * counts its own settings; before that, and for a pane, its words.
 */
export function matchesIn(section: SettingsSection, query: string, list: SettingsList | null = null): number {
  if (section.schema === true && list !== null) {
    return settingsIn(list, section.id).filter((one) => matches(wordsOf(one), query)).length;
  }
  return section.settings.filter((words) => matches([section.label, ...words], query)).length;
}

// ---- Which group is open --------------------------------------------------------------------

let open = "agents";
/** The section the palette asked for, scrolled to once its group is drawn. */
let focus: SettingsSectionId | null = null;
/** How many times the palette has asked, so a group already open still scrolls to the section. */
let asked = 0;
const listeners = new Set<() => void>();

/**
 * Open Settings at a category, from the palette's row id: `settings:keyboard`, or `fleet_settings`.
 * Its group opens and the section is scrolled to.
 */
export function openSettingsAt(id: string, go: () => void): void {
  const wanted = id === "fleet_settings" ? "limits" : id.replace(/^settings:/, "");
  const found = SETTINGS_SECTIONS.find((one) => one.id === wanted);
  if (found === undefined) return;
  focus = found.id;
  asked += 1;
  open = shippedGroupOf(found.id) ?? found.id;
  listeners.forEach((on) => on());
  go();
}

export function setSettingsSection(id: string): void {
  if (open === id) return;
  open = id;
  focus = null;
  listeners.forEach((on) => on());
}

/** The section to scroll to, once: reading it clears it. */
export function takeSettingsFocus(): SettingsSectionId | null {
  const wanted = focus;
  focus = null;
  return wanted;
}

/** Changes each time the palette opens a section, so a screen knows to look for its focus again. */
export function useSettingsAsked(): number {
  return useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    () => asked,
    () => asked,
  );
}

/** The group Settings draws, and the way to choose another. */
export function useSettingsSection(): [string, (id: string) => void] {
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
  id: one.id === "limits" ? "fleet_settings" : `settings:${one.id}`,
  label: one.palette,
  aliases: ["settings", "preferences", one.label, ...one.settings.flat()],
}));
