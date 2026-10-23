// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.
// Themes and layouts come from their own sources. Tweaks, Words and Fleet are what a teaching made, from the taught source, and a
// mod that was taught opens to what it holds and what it was taught on. `all` is every kind in one list, drawn from the same rows:
// the mods that are on first, those switched off under them, each kept in its kind's order.

import { useLayoutEffect, useRef, type ReactElement } from "react";
import { Card, CardContent, CardHeader, CardTitle, ModRow, ModRules } from "@armada/components";
import type { ModKind, ModTracker } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { taughtOf, useTaught } from "./taught-source";
import type { TaughtKind, TaughtMod } from "./taught-source";
import { useThemes } from "./theme-source";

export type ModListKind = "theme" | "layout" | "all" | TaughtKind;

/** A tracker mod is used only while it is tried, valid and switched on: a refusal or a failed try frames it red, a mod still to try amber. */
function trackerStanding(mod: TaughtMod, tracker: ModTracker): "used" | "waiting" | "refused" | "off" {
  if (!tracker.valid || tracker.tried.state === "failed") return "refused";
  if (!mod.enabled) return "off";
  return tracker.tried.state === "passed" ? "used" : "waiting";
}

const ROW_KIND: Record<TaughtKind, ModKind> = { words: "words", fleet: "fleet", tweak: "tweak", tracker: "tracker" };
const KIND_ORDER: readonly ModKind[] = ["theme", "tweak", "layout", "words", "fleet", "tracker"];

/** The frame a row in a list that holds a tracker is drawn in, so every row there takes the same padding: refused, off or in use. */
const standingOf = (mod: { enabled: boolean; problem?: string }): "used" | "refused" | "off" => (mod.problem !== undefined ? "refused" : mod.enabled ? "used" : "off");

type Listed = { key: string; kind: ModKind; name: string; enabled: boolean; row: ReactElement };

export function ModList({ kind, title }: { kind: ModListKind; title?: string }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const taught = useTaught();
  const { source, details, opened } = taught;
  // The row a switch was pressed on keeps focus when it moves to the other half of the list.
  const pressed = useRef<string | undefined>(undefined);
  const header =
    title === undefined ? null : (
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
    );

  // A list that holds a tracker keeps the columns of its marks and Try in every row, and frames every row in its standing.
  const lined = (kind === "all" || kind === "tracker") && taught.mods.some((one) => one.kind === "tracker");

  const taughtRow = (mod: TaughtMod): Listed => {
    const line = [mod.description, mod.on === "" ? undefined : `Taught on ${mod.on}`].filter((part) => part !== undefined).join(". ");
    return {
      key: `${mod.kind}:${mod.name}`,
      kind: ROW_KIND[mod.kind],
      name: mod.name,
      enabled: mod.enabled,
      row: (
        <ModRow
          key={`${mod.kind}:${mod.name}`}
          title={mod.name}
          kind={ROW_KIND[mod.kind]}
          enabled={mod.enabled}
          onEnabled={(on) => {
            pressed.current = mod.name;
            source.setEnabled(mod.name, on);
          }}
          detail={mod.branch === undefined ? line : `${line} (${mod.branch})`}
          promoteDisabled={mod.branch !== undefined}
          onPromote={() => source.promote(mod.name)}
          {...(lined ? { withTracker: true, standing: mod.tracker === undefined ? standingOf(mod) : trackerStanding(mod, mod.tracker) } : {})}
          {...(mod.tracker === undefined ? {} : { tracker: mod.tracker })}
          {...(mod.tracker === undefined || source.tryIt === undefined ? {} : { onTry: () => source.tryIt?.(mod.name) })}
          // A mod opens only when it holds something to read: Fleet's list says a real mod's kind and what it was taught on, no more.
          {...(mod.parts.length === 0 ? {} : { opened: opened === mod.name, onOpened: (open: boolean) => source.open(open ? mod.name : undefined) })}
          autoFocus={opened === mod.name}
        >
          {mod.parts.length === 0 ? undefined : <ModRules name={mod.name} parts={mod.parts} />}
        </ModRow>
      ),
    };
  };

  const ownRow = (as: "theme" | "layout", mod: { name: string; title: string; enabled: boolean; problem?: string; branch?: string }): Listed => {
    const { enable, promote } = as === "theme" ? { enable: themes.source.setEnabled, promote: themes.source.promote } : { enable: layouts.source.setEnabled, promote: layouts.source.promote };
    // A layout a teaching made opens to what it hides and what it was taught on.
    const own = as === "layout" ? details[mod.name] : undefined;
    const hides = own?.parts.flatMap((part) => part.facts).find((fact) => fact.label === "Hides")?.value;
    const line = mod.problem ?? mod.branch ?? (own === undefined ? undefined : `${hides === undefined ? "" : `Hides ${hides}. `}Taught on ${own.on}`);
    return {
      key: `${as}:${mod.name}`,
      kind: as,
      name: mod.title,
      enabled: mod.enabled,
      row: (
        <ModRow
          key={`${as}:${mod.name}`}
          title={mod.title}
          kind={as}
          enabled={mod.enabled}
          onEnabled={(on) => {
            pressed.current = mod.title;
            enable(mod.name, on);
          }}
          {...(line === undefined ? {} : { detail: line })}
          {...(lined ? { withTracker: true, standing: standingOf(mod) } : {})}
          promoteDisabled={mod.problem !== undefined || mod.branch !== undefined}
          onPromote={() => promote(mod.name)}
          {...(own === undefined
            ? {}
            : { opened: opened === mod.name, onOpened: (open: boolean) => source.open(open ? mod.name : undefined), autoFocus: opened === mod.name })}
        >
          {own === undefined ? undefined : <ModRules name={mod.title} parts={own.parts} />}
        </ModRow>
      ),
    };
  };

  const byName = (a: Listed, b: Listed) => a.name.localeCompare(b.name);
  let rows: Listed[];
  if (kind === "all") {
    rows = [
      // A theme is applied once it is chosen, so the one in force is the one listed.
      ...themes.mods.filter((mod) => mod.name === themes.active).map((mod) => ownRow("theme", mod)),
      ...taught.mods.map(taughtRow),
      ...layouts.mods.map((mod) => ownRow("layout", mod)),
    ].sort((a, b) => Number(b.enabled) - Number(a.enabled) || KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) || byName(a, b));
  } else if (kind !== "theme" && kind !== "layout") {
    rows = taughtOf(taught, kind).map(taughtRow).sort(byName);
  } else {
    rows = (kind === "theme" ? themes.mods.map((mod) => ownRow("theme", mod)) : layouts.mods.map((mod) => ownRow("layout", mod))).sort(byName);
  }

  useLayoutEffect(() => {
    const name = pressed.current;
    if (name === undefined) return;
    pressed.current = undefined;
    const held = document.activeElement;
    if (held !== null && held !== document.body) return;
    document.querySelector<HTMLInputElement>(`[role="group"][aria-label="${CSS.escape(name)}"] input[role="switch"]`)?.focus();
  });

  if (rows.length === 0) return null;
  return (
    <Card>
      {header}
      <CardContent>
        {rows.map((one) => one.row)}
      </CardContent>
    </Card>
  );
}
