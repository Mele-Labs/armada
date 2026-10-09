// Settings → Theme: Dark and Light, the catalogue's themes under whichever of the two their
// background is, and the themes mods ship that are on and pass their checks.

import { ThemePicker } from "@armada/components";
import type { ThemeGroup } from "@armada/components";

import { offered, useThemes } from "./theme-source";

const byTitle = (a: { title: string }, b: { title: string }) => a.title.localeCompare(b.title);

export function ThemeSettings() {
  const { mods, catalogue, active, source } = useThemes();
  const of = (tone: "dark" | "light") => catalogue.filter((one) => one.tone === tone).sort(byTitle).map(({ id, title }) => ({ id, title }));
  const groups: ThemeGroup[] = [
    { label: "Dark", choices: [{ id: "dark", title: "Dark" }, ...of("dark")] },
    { label: "Light", choices: [{ id: "light", title: "Light" }, ...of("light")] },
    { label: "From mods", choices: mods.filter(offered).map((one) => ({ id: one.name, title: one.title })) },
  ];
  return <ThemePicker label="Theme" value={active} groups={groups} onValue={(id) => source.setActive(id)} />;
}
