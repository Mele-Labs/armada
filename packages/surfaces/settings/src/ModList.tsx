// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if
// anything, and Promote. Drawn under Theme and Layout, which they change, and in Rules, which they are.
// A rules mod opens to the rules it holds.

import { Card, CardContent, CardHeader, CardTitle, ModRow, ModRules } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { useRules } from "./rules-source";
import { useThemes } from "./theme-source";

export function ModList({ kind, title }: { kind: "theme" | "layout" | "rules"; title?: string }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const rules = useRules();
  if (kind === "rules") {
    const { source, opened } = rules;
    if (rules.mods.length === 0) return null;
    return (
      <Card>
        {title === undefined ? null : (
          <CardHeader>
            <CardTitle>{title}</CardTitle>
          </CardHeader>
        )}
        <CardContent>
          {[...rules.mods]
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((mod) => (
              <ModRow
                key={mod.name}
                title={mod.name}
                kind="rules"
                enabled={mod.enabled}
                onEnabled={(on) => source.setEnabled(mod.name, on)}
                detail={mod.branch === undefined ? mod.description : `${mod.description} (${mod.branch})`}
                promoteDisabled={mod.branch !== undefined}
                onPromote={() => source.promote(mod.name)}
                opened={opened === mod.name}
                onOpened={(open) => source.open(open ? mod.name : undefined)}
                autoFocus={opened === mod.name}
              >
                <ModRules name={mod.name} parts={mod.rules} />
              </ModRow>
            ))}
        </CardContent>
      </Card>
    );
  }
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
