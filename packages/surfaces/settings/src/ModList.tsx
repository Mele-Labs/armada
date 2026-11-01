// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.
// Themes and layouts come from their own sources. Tweaks, Words and Fleet are what a teaching made, from the taught source, and a
// mod that was taught opens to what it holds and what it was taught on. `all` is every kind in one list, drawn from the same rows:
// the mods that are on first, those switched off under them, each kept in its kind's order. Against a shared home each theme and
// layout carries Kit's own mark and act for it, and the mods only the home has are listed after, so this list and Kit's Mods offer
// the same acts.

import { useLayoutEffect, useRef, useState, type ReactElement } from "react";
import { Button, Card, CardContent, CardHeader, CardTitle, KIT_KIND_LABEL, ModRow, ModRules, SyncAct, SyncMarks, SyncSheet, useSharedHome } from "@armada/components";
import type { ModKind, ModTracker, SharedRow } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { taughtOf, useTaught } from "./taught-source";
import type { TaughtKind, TaughtMod } from "./taught-source";
import { useThemes } from "./theme-source";

type Mod = {
  name: string;
  title: string;
  enabled: boolean;
  problem?: string;
  branch?: string;
  pull?: { number: number; title: string; address: string };
  job?: { id: string; title: string };
  builtIn?: boolean;
};

/** A mod only the home has is named by its folder; its title is that name as a person reads it. */
const titleOf = (name: string): string => name.charAt(0).toUpperCase() + name.slice(1);

export type ModListKind = "theme" | "layout" | "all" | TaughtKind;

/** A tracker mod is used only while it is tried, valid and switched on: a refusal or a failed try frames it red, a mod still to try amber. */
function trackerStanding(mod: TaughtMod, tracker: ModTracker): "used" | "waiting" | "refused" | "off" {
  if (!tracker.valid || tracker.tried.state === "failed") return "refused";
  if (!mod.enabled) return "off";
  return tracker.tried.state === "passed" ? "used" : "waiting";
}

const ROW_KIND: Record<TaughtKind, ModKind> = { words: "words", fleet: "fleet", tweak: "tweak", tracker: "tracker", tab: "tab" };
const KIND_ORDER: readonly ModKind[] = ["theme", "tweak", "layout", "words", "fleet", "tracker", "tab"];

/** The frame a row in a list that holds a tracker is drawn in, so every row there takes the same padding: refused, off or in use. */
const standingOf = (mod: { enabled: boolean; problem?: string }): "used" | "refused" | "off" => (mod.problem !== undefined ? "refused" : mod.enabled ? "used" : "off");

type Listed = { key: string; kind: ModKind; name: string; enabled: boolean; row: ReactElement };

/** The mod a press on a row names: its folder, its title as the row draws it, and its kind. */
export type ModOpened = { name: string; title: string; kind: ModKind };

/** `onCreate` draws Create mod in the header, which starts a Mod Job from a description. `onOpenMod` opens a mod from a press on its row, anywhere but its switch. */
export function ModList({ kind, title, onOpenPull, onOpenJob, onCreate, onOpenMod }: { kind: ModListKind; title?: string; onOpenPull?: (address: string) => void; onOpenJob?: (id: string) => void; onCreate?: () => void; onOpenMod?: (mod: ModOpened) => void }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const taught = useTaught();
  const { source, details, opened } = taught;
  const shared = useSharedHome();
  const [comparing, setComparing] = useState<SharedRow | null>(null);
  // The row a switch was pressed on keeps focus when it moves to the other half of the list.
  const pressed = useRef<string | undefined>(undefined);
  const header =
    title === undefined ? null : (
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        {onCreate === undefined ? null : (
          <Button variant="secondary" size="sm" ground="card" onClick={onCreate}>
            Create mod
          </Button>
        )}
      </CardHeader>
    );
  const themeMods = themes.mods as readonly Mod[];
  const layoutMods = layouts.mods as readonly Mod[];
  const homeRows = shared?.rows.filter((row) => row.kind === "mods") ?? [];
  const homeOnly = homeRows.filter((row) => {
    const as = row.modKind ?? "theme";
    const local = as === "theme" ? themeMods : as === "layout" ? layoutMods : taught.mods.filter((mod) => mod.kind === as);
    return row.state === "home" && (kind === "all" || as === kind) && !local.some((mod) => !("builtIn" in mod && mod.builtIn === true) && mod.name === row.name);
  });
  const sync = (row: SharedRow | undefined) =>
    row === undefined || shared === undefined
      ? {}
      : { syncMark: <SyncMarks row={row} onCompare={setComparing} />, syncAct: <SyncAct row={row} acts={shared} /> };

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
          {...(onOpenMod === undefined ? {} : { onOpen: () => onOpenMod({ name: mod.name, title: mod.name, kind: ROW_KIND[mod.kind] }) })}
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

  const ownRow = (as: "theme" | "layout", mod: Mod): Listed => {
    const { enable, promote } = as === "theme" ? { enable: themes.source.setEnabled, promote: themes.source.promote } : { enable: layouts.source.setEnabled, promote: layouts.source.promote };
    // A layout a teaching made opens to what it hides and what it was taught on.
    const own = as === "layout" ? details[mod.name] : undefined;
    const hides = own?.parts.flatMap((part) => part.facts).find((fact) => fact.label === "Hides")?.value;
    const line = mod.problem ?? (mod.pull === undefined ? mod.branch : undefined) ?? (own === undefined ? undefined : `${hides === undefined ? "" : `Hides ${hides}. `}Taught on ${own.on}`);
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
          {...(onOpenMod === undefined ? {} : { onOpen: () => onOpenMod({ name: mod.name, title: mod.title, kind: as }) })}
          builtIn={mod.builtIn === true}
          {...sync(mod.builtIn === true ? undefined : homeRows.find((row) => row.name === mod.name))}
          {...(mod.pull === undefined || onOpenPull === undefined ? {} : { pull: { number: mod.pull.number, title: mod.pull.title, onOpen: () => onOpenPull(mod.pull!.address) } })}
          {...(mod.job === undefined || onOpenJob === undefined ? {} : { job: { title: mod.job.title, onOpen: () => onOpenJob(mod.job!.id) } })}
          {...(lined ? { withTracker: true, standing: standingOf(mod) } : {})}
          promoteDisabled={mod.problem !== undefined || mod.branch !== undefined || mod.pull !== undefined}
          {...(mod.builtIn === true ? {} : { onPromote: () => promote(mod.name) })}
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
  // A mod Armada ships before a local one of the same name; the sort by name after keeps that order, being stable.
  const shippedFirst = (mods: readonly Mod[]) => [...mods].sort((a, b) => Number(b.builtIn === true) - Number(a.builtIn === true));
  let rows: Listed[];
  if (kind === "all") {
    rows = [
      // A theme is applied once it is chosen, so the one in force is the one listed.
      ...themeMods.filter((mod) => mod.name === themes.active).map((mod) => ownRow("theme", mod)),
      ...taught.mods.map(taughtRow),
      ...layoutMods.map((mod) => ownRow("layout", mod)),
    ].sort((a, b) => Number(b.enabled) - Number(a.enabled) || KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) || byName(a, b));
  } else if (kind !== "theme" && kind !== "layout") {
    rows = taughtOf(taught, kind).map(taughtRow).sort(byName);
  } else {
    rows = (kind === "theme" ? shippedFirst(themeMods).map((mod) => ownRow("theme", mod)) : shippedFirst(layoutMods).map((mod) => ownRow("layout", mod))).sort(byName);
  }

  useLayoutEffect(() => {
    const name = pressed.current;
    if (name === undefined) return;
    pressed.current = undefined;
    const held = document.activeElement;
    if (held !== null && held !== document.body) return;
    document.querySelector<HTMLInputElement>(`[role="group"][aria-label="${CSS.escape(name)}"] input[role="switch"]`)?.focus();
  });

  // With Create mod to draw, a machine with no mods yet still shows the page it is made from.
  if (rows.length === 0 && homeOnly.length === 0 && onCreate === undefined) return null;
  return (
    <Card>
      {header}
      <CardContent>
        {rows.map((one) => one.row)}
        {homeOnly.map((row) => (
          <ModRow key={`home:${row.name}`} title={titleOf(row.name)} kind={row.modKind ?? "theme"} withTracker={lined} {...sync(row)} />
        ))}
      </CardContent>
      {shared === undefined || comparing === null ? null : <SyncSheet row={comparing} kindLabel={KIT_KIND_LABEL.mods} acts={shared} onClose={() => setComparing(null)} />}
    </Card>
  );
}
