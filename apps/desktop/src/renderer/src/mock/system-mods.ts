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
