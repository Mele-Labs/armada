// Settings → Theme: Dark and Light, the catalogue's themes under whichever of the two their
// background is, and the themes mods ship that are on and pass their checks.

import { ThemePicker } from "@armada/components";
import type { ThemeChoice, ThemeGroup } from "@armada/components";

import { offered, useThemes } from "./theme-source";

const byTitle = (a: { title: string }, b: { title: string }) => a.title.localeCompare(b.title);

export function ThemeSettings() {
  const { mods, catalogue, active, swatches, source } = useThemes();
  const shown = (id: string, title: string, swatch: readonly string[] | undefined): ThemeChoice => ({ id, title, ...(swatch === undefined ? {} : { swatch }) });
  const of = (tone: "dark" | "light") => catalogue.filter((one) => one.tone === tone).sort(byTitle).map((one) => shown(one.id, one.title, one.swatch));
  const groups: ThemeGroup[] = [
    { label: "Dark", choices: [shown("dark", "Dark", swatches?.["dark"]), ...of("dark")] },
    { label: "Light", choices: [shown("light", "Light", swatches?.["light"]), ...of("light")] },
    { label: "From mods", choices: mods.filter(offered).map((one) => shown(one.name, one.title, one.swatch)) },
  ];
  return <ThemePicker label="Theme" value={active} groups={groups} onValue={(id) => source.setActive(id)} />;
}
