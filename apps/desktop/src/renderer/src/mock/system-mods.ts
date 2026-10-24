// Mock-only: the mods on a machine in the `system-sidebar` scenario, so Theme and Layout draw some
// under their settings. Installed once the window has reset the sources, which would otherwise clear them.

import { parseLayout } from "@armada/shell";

import { mockLayout, TIDY_DASHBOARD } from "./layout";
import { DUSK_MOD, mockThemes } from "./themes";

export function installSystemMods(): void {
  mockThemes.install(DUSK_MOD);
  mockThemes.install({ ...DUSK_MOD, name: "harbour", title: "Harbour", enabled: false });
  mockThemes.install({ ...DUSK_MOD, name: "slate", title: "Slate", enabled: false, problem: "theme.css has no --bg-raised" });
  mockLayout.install({ name: "tidy", title: "Tidy", enabled: false, file: parseLayout(TIDY_DASHBOARD).file });
}

/** The `mods-travel` machine: Dusk, the Harbour Armada ships beside a local one of the same name, and the Tidy layout. */
export function installTravelMods(): void {
  mockThemes.install(DUSK_MOD);
  mockThemes.install({ ...DUSK_MOD, name: "built-in:harbour", title: "Harbour", enabled: true, builtIn: true });
  mockThemes.install({ ...DUSK_MOD, name: "harbour", title: "Harbour", enabled: false });
  mockLayout.install({ name: "tidy", title: "Tidy", enabled: false, file: parseLayout(TIDY_DASHBOARD).file });
}

/** A mod only the shared home had, brought to this machine. A mod nobody switched is on. */
export function installBroughtMod(kind: "theme" | "layout", name: string): void {
  const title = name.charAt(0).toUpperCase() + name.slice(1);
  if (kind === "layout") mockLayout.install({ name, title, enabled: true, file: parseLayout(TIDY_DASHBOARD).file });
  else mockThemes.install({ ...DUSK_MOD, name, title, enabled: true });
}
