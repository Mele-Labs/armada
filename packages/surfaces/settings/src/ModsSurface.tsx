// Mods on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.

import { Card, CardContent, ModRow } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { useThemes } from "./theme-source";

export function ModsSurface() {
  const { mods, source } = useThemes();
  const layouts = useLayouts();
  const rows = [
    ...mods.map((mod) => ({ kind: "theme" as const, mod, enable: source.setEnabled, promote: source.promote })),
    ...layouts.mods.map((mod) => ({ kind: "layout" as const, mod, enable: layouts.source.setEnabled, promote: layouts.source.promote })),
  ].sort((a, b) => a.mod.name.localeCompare(b.mod.name));
  return (
    <div className="armada-screen__pane">
      {rows.map(({ kind, mod, enable, promote }) => (
        <Card key={`${kind}:${mod.name}`}>
          <CardContent>
            <ModRow
              title={mod.title}
              kind={kind}
              enabled={mod.enabled}
              onEnabled={(on) => enable(mod.name, on)}
              {...(mod.problem ?? mod.branch ? { detail: mod.problem ?? mod.branch } : {})}
              promoteDisabled={mod.problem !== undefined || mod.branch !== undefined}
              onPromote={() => promote(mod.name)}
            />
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
