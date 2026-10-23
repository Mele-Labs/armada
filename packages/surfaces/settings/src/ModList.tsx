// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.
// Themes and layouts come from their own sources. Tweaks, Words and Fleet are what a teaching made, from the taught source, and a
// mod that was taught opens to what it holds and what it was taught on.

import { Card, CardContent, CardHeader, CardTitle, ModRow, ModRules } from "@armada/components";
import type { ModKind } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { taughtOf, useTaught } from "./taught-source";
import type { TaughtKind } from "./taught-source";
import { useThemes } from "./theme-source";

export type ModListKind = "theme" | "layout" | TaughtKind;

const ROW_KIND: Record<TaughtKind, ModKind> = { words: "words", fleet: "fleet", tweak: "tweak" };

export function ModList({ kind, title }: { kind: ModListKind; title?: string }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const taught = useTaught();
  const { source, details, opened } = taught;
  const header =
    title === undefined ? null : (
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
    );
  if (kind === "words" || kind === "fleet" || kind === "tweak") {
    const mods = taughtOf(taught, kind);
    if (mods.length === 0) return null;
    return (
      <Card>
        {header}
        <CardContent>
          {[...mods]
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((mod) => (
              <ModRow
                key={mod.name}
                title={mod.name}
                kind={ROW_KIND[mod.kind]}
                enabled={mod.enabled}
                onEnabled={(on) => source.setEnabled(mod.name, on)}
                detail={mod.branch === undefined ? mod.description : `${mod.description} (${mod.branch})`}
                promoteDisabled={mod.branch !== undefined}
                onPromote={() => source.promote(mod.name)}
                opened={opened === mod.name}
                onOpened={(open) => source.open(open ? mod.name : undefined)}
                autoFocus={opened === mod.name}
              >
                <ModRules name={mod.name} parts={mod.parts} />
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
        {sorted.map((mod) => {
          // A layout a teaching made opens to what it hides and what it was taught on.
          const own = kind === "layout" ? details[mod.name] : undefined;
          const line = mod.problem ?? mod.branch ?? (own === undefined ? undefined : `Taught on ${own.on}`);
          return (
            <ModRow
              key={mod.name}
              title={mod.title}
              kind={kind}
              enabled={mod.enabled}
              onEnabled={(on) => enable(mod.name, on)}
              {...(line === undefined ? {} : { detail: line })}
              promoteDisabled={mod.problem !== undefined || mod.branch !== undefined}
              onPromote={() => promote(mod.name)}
              {...(own === undefined
                ? {}
                : { opened: opened === mod.name, onOpened: (open: boolean) => source.open(open ? mod.name : undefined), autoFocus: opened === mod.name })}
            >
              {own === undefined ? undefined : <ModRules name={mod.title} parts={own.parts} />}
            </ModRow>
          );
        })}
      </CardContent>
    </Card>
  );
}
