// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if
// anything, and Promote. Drawn under Theme and Layout, which they change.

import { Card, CardContent, ModRow } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { useThemes } from "./theme-source";

export function ModList({ kind }: { kind: "theme" | "layout" }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const { mods, enable, promote } =
    kind === "theme"
      ? { mods: themes.mods, enable: themes.source.setEnabled, promote: themes.source.promote }
      : { mods: layouts.mods, enable: layouts.source.setEnabled, promote: layouts.source.promote };
  if (mods.length === 0) return null;
  const sorted = [...mods].sort((a, b) => a.name.localeCompare(b.name));
  return (
    <Card>
      <CardContent>
        {sorted.map((mod) => (
          <ModRow
            key={mod.name}
            title={mod.title}
            kind={kind}
            enabled={mod.enabled}
            onEnabled={(on) => enable(mod.name, on)}
            {...(mod.problem ?? mod.branch ? { detail: mod.problem ?? mod.branch } : {})}
            promoteDisabled={mod.problem !== undefined || mod.branch !== undefined}
            onPromote={() => promote(mod.name)}
          />
        ))}
      </CardContent>
    </Card>
  );
}
