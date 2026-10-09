// Mods on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.

import { Card, CardContent, ModRow } from "@armada/components";

import { useThemes } from "./theme-source";

export function ModsSurface() {
  const { mods, source } = useThemes();
  return (
    <div className="armada-screen__pane">
      {mods.map((mod) => (
        <Card key={mod.name}>
          <CardContent>
            <ModRow
              title={mod.title}
              enabled={mod.enabled}
              onEnabled={(on) => source.setEnabled(mod.name, on)}
              {...(mod.problem ?? mod.branch ? { detail: mod.problem ?? mod.branch } : {})}
              promoteDisabled={mod.problem !== undefined || mod.branch !== undefined}
              onPromote={() => source.promote(mod.name)}
            />
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
