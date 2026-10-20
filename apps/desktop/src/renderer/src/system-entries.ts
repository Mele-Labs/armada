// The entries of the System sidebar, the settings words that find them, and the old surfaces each one
// stands in for, so a palette row or a key that opened Kit, Mods, Settings, Guides or Workflows
// still lands on the right entry. The sidebar itself is `system-surface.tsx`.

import { holds, type KitKindId } from "@armada/components";
import { SETTINGS_SECTIONS } from "@armada/settings";
import { SURFACE } from "@armada/shell";

export type SystemGroupId = "kit" | "fleet" | "bridge";

export type SystemEntryDef = {
  id: string;
  label: string;
  group: SystemGroupId;
  /** A kind drawn as a list with the picked item beside it. */
  kit?: KitKindId;
  /** The settings.json categories it draws, by id (`@armada/settings` sections). */
  sections?: readonly string[];
};

export const SYSTEM_GROUPS: readonly { id: SystemGroupId; label: string }[] = [
  { id: "kit", label: "Kit" },
  { id: "fleet", label: "Fleet" },
  { id: "bridge", label: "Bridge" },
];

export const SYSTEM_ENTRIES: readonly SystemEntryDef[] = [
  { id: "kit:skills", label: "Skills", group: "kit", kit: "skills" },
  { id: "kit:plugins", label: "Plugins", group: "kit", kit: "plugins" },
  { id: "kit:sub-agents", label: "Sub agents", group: "kit", kit: "sub-agents" },
  { id: "kit:commands", label: "Commands", group: "kit", kit: "commands" },
  { id: "kit:servers", label: "Servers", group: "kit", kit: "servers" },
  { id: "kit:instructions", label: "Instructions", group: "kit", kit: "instructions" },
  { id: "kit:output-styles", label: "Output styles", group: "kit", kit: "output-styles" },
  { id: "kit:allowlist", label: "Allowlist", group: "kit", kit: "allowlist" },
  { id: "kit:workflows", label: "Workflows", group: "kit" },
  { id: "kit:models", label: "Models", group: "kit", sections: ["model", "effort"] },
  { id: "kit:prompts", label: "Prompts", group: "kit", sections: ["prompts"] },
  { id: "kit:harness", label: "Harness", group: "kit", sections: ["harness"] },
  { id: "fleet:limits", label: "Limits", group: "fleet", sections: ["limits"] },
  { id: "fleet:timeouts", label: "Timeouts", group: "fleet", sections: ["timeouts"] },
  { id: "fleet:retention", label: "Retention", group: "fleet", sections: ["retention"] },
  { id: "fleet:plan", label: "Plan", group: "fleet", sections: ["plan"] },
  { id: "fleet:machine", label: "Machine", group: "fleet", sections: ["machine"] },
  { id: "fleet:account", label: "Account", group: "fleet", sections: ["account"] },
  { id: "fleet:store", label: "Store", group: "fleet", sections: ["store"] },
  { id: "fleet:features", label: "Features", group: "fleet", sections: ["features"] },
  { id: "bridge:theme", label: "Theme", group: "bridge", sections: ["theme"] },
  { id: "bridge:layout", label: "Layout", group: "bridge", sections: ["layout"] },
  { id: "bridge:keyboard", label: "Keyboard shortcuts", group: "bridge", sections: ["keyboard"] },
  { id: "bridge:time", label: "Time", group: "bridge", sections: ["time"] },
  { id: "bridge:editor", label: "Editor", group: "bridge", sections: ["editor"] },
  { id: "bridge:terminal", label: "Terminal", group: "bridge", sections: ["terminal"] },
  { id: "bridge:links", label: "Links", group: "bridge", sections: ["links"] },
  { id: "bridge:phone", label: "Phone", group: "bridge", sections: ["phone"] },
  { id: "bridge:guides", label: "Guides", group: "bridge", sections: ["guides"] },
];

export const FIRST_ENTRY = "kit:skills";

export const entryOf = (id: string): SystemEntryDef | undefined => SYSTEM_ENTRIES.find((one) => one.id === id);

/** The entry that draws a settings.json category, for a palette row that opens Settings at one. */
export const entryOfSection = (section: string): string | undefined => SYSTEM_ENTRIES.find((one) => one.sections?.includes(section))?.id;

/** The entry a palette row that opens Settings at a category opens: `settings:keyboard`, or `fleet_settings` for Limits. */
export const entryOfSettingsRow = (row: string): string | undefined => entryOfSection(row === "fleet_settings" ? "limits" : row.replace(/^settings:/, ""));

/** The entry that lists one kind, as a plugin's row for something it brings leads to it. */
export const entryOfKind = (kind: KitKindId): string => SYSTEM_ENTRIES.find((one) => one.kit === kind)?.id ?? FIRST_ENTRY;

/**
 * The entry a surface the window used to have opens: Mods is Theme, Guides and Workflows are their own, and Kit and
 * Settings keep the entry last used when it is one of theirs. `last` is that entry.
 */
export function entryForSurface(surface: string, last: string): string | undefined {
  const now = entryOf(last);
  switch (surface) {
    case SURFACE.mods:
      return "bridge:theme";
    case SURFACE.guides:
      return "bridge:guides";
    case SURFACE.workflows:
      return "kit:workflows";
    case SURFACE.kit:
      return now?.kit !== undefined ? last : FIRST_ENTRY;
    case SURFACE.settings:
      return now?.sections !== undefined ? last : "fleet:limits";
    default:
      return undefined;
  }
}

/** The surface the System tab opens for an entry: the old one it stands in for, which `entryForSurface` maps back to the same entry. */
export function surfaceOfEntry(id: string): string {
  if (id === "kit:workflows") return SURFACE.workflows;
  if (id === "bridge:guides") return SURFACE.guides;
  return entryOf(id)?.kit !== undefined ? SURFACE.kit : SURFACE.settings;
}

/** The words a search finds an entry by: its name, and every word of the settings it holds. */
function wordsOf(entry: SystemEntryDef): string {
  const sections = (entry.sections ?? []).flatMap((id) => SETTINGS_SECTIONS.filter((one) => one.id === id));
  return [entry.label, ...sections.flatMap((one) => [one.label, one.palette, ...one.settings.flat()])].join(" ");
}

/**
 * How a search finds an entry: by its name or settings words, by an item in its list (`items`, so its list opens
 * filtered), or not at all.
 */
export function foundBy(entry: SystemEntryDef, query: string, names: (kind: KitKindId) => string[]): "words" | "items" | null {
  if (query.trim() === "") return "words";
  if (holds(wordsOf(entry), query)) return "words";
  if (entry.kit !== undefined && names(entry.kit).some((name) => holds(name, query))) return "items";
  return null;
}
