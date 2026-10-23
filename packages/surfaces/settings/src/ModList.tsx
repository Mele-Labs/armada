// The mods of one kind on this machine: one row each, with whether it is on, what is wrong with it if
// anything, and Promote. Drawn under Theme and Layout, which they change. Against a shared home each row
// carries Kit's own mark and act for it, and the mods only the home has are listed after, so this list and
// Kit's Mods offer the same acts.

import { useState } from "react";
import { Card, CardContent, KIT_KIND_LABEL, ModRow, SyncAct, SyncMarks, SyncSheet, useSharedHome } from "@armada/components";
import type { SharedRow } from "@armada/components";
import { useLayouts } from "@armada/shell";

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

export function ModList({ kind, onOpenPull, onOpenJob }: { kind: "theme" | "layout"; onOpenPull?: (address: string) => void; onOpenJob?: (id: string) => void }) {
  const themes = useThemes();
  const layouts = useLayouts();
  const shared = useSharedHome();
  const [comparing, setComparing] = useState<SharedRow | null>(null);
  const { mods, enable, promote } =
    kind === "theme"
      ? { mods: themes.mods as readonly Mod[], enable: themes.source.setEnabled, promote: themes.source.promote }
      : { mods: layouts.mods as readonly Mod[], enable: layouts.source.setEnabled, promote: layouts.source.promote };
  const rows = shared?.rows.filter((row) => row.kind === "mods") ?? [];
  const homeOnly = rows.filter((row) => row.state === "home" && (row.modKind ?? "theme") === kind && !mods.some((mod) => mod.builtIn !== true && mod.name === row.name));
  if (mods.length === 0 && homeOnly.length === 0) return null;
  // By title, a mod Armada ships before a local one of the same name.
  const sorted = [...mods].sort((a, b) => a.title.localeCompare(b.title) || Number(b.builtIn === true) - Number(a.builtIn === true));
  const sync = (row: SharedRow | undefined) =>
    row === undefined || shared === undefined
      ? {}
      : {
          sync: (
            <>
              <SyncMarks row={row} onCompare={setComparing} />
              <SyncAct row={row} acts={shared} />
            </>
          ),
        };
  return (
    <Card>
      <CardContent>
        {sorted.map((mod) => {
          const detail = mod.problem ?? (mod.pull === undefined ? mod.branch : undefined);
          return (
            <ModRow
              key={mod.name}
              title={mod.title}
              kind={kind}
              enabled={mod.enabled}
              onEnabled={(on) => enable(mod.name, on)}
              {...(detail === undefined ? {} : { detail })}
              builtIn={mod.builtIn === true}
              {...sync(mod.builtIn === true ? undefined : rows.find((row) => row.name === mod.name))}
              {...(mod.pull === undefined || onOpenPull === undefined ? {} : { pull: { number: mod.pull.number, title: mod.pull.title, onOpen: () => onOpenPull(mod.pull!.address) } })}
              {...(mod.job === undefined || onOpenJob === undefined ? {} : { job: { title: mod.job.title, onOpen: () => onOpenJob(mod.job!.id) } })}
              promoteDisabled={mod.problem !== undefined || mod.branch !== undefined || mod.pull !== undefined}
              {...(mod.builtIn === true ? {} : { onPromote: () => promote(mod.name) })}
            />
          );
        })}
        {homeOnly.map((row) => (
          <ModRow key={`home:${row.name}`} title={titleOf(row.name)} kind={kind} {...sync(row)} />
        ))}
      </CardContent>
      {shared === undefined || comparing === null ? null : <SyncSheet row={comparing} kindLabel={KIT_KIND_LABEL.mods} acts={shared} onClose={() => setComparing(null)} />}
    </Card>
  );
}
