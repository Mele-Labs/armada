// What a taught Logbook entry also changes, for the line under its text: a line for each mod the teaching made, Words or Fleet, read
// from Fleet's mod list. Forget on the entry has Fleet remove its mods, and with them every line here.

import { useMemo } from "react";
import type { ReactNode } from "react";
import { AlsoChanges } from "@armada/components";
import type { AlsoChangesProps } from "@armada/components";
import type { LogbookEntry } from "@armada/protocol";

import { useModList } from "./mod-list";
import { askToOpenMod } from "./open-mod";
import { teachFlavour } from "./teach";
import { useTaughtMods } from "./taught-entries";

type Line = AlsoChangesProps["lines"][number];

const PART = { words: "Words", fleet: "Fleet" } as const;

function Also({ entry }: { entry: LogbookEntry }) {
  const made = useTaughtMods(entry.id);
  const listed = useModList();
  const lines = useMemo(
    () =>
      (["words", "fleet"] as const).flatMap((kind): Line[] => {
        // A teaching makes at most one mod of a kind, so the line is the newest listed.
        const row = listed.filter((one) => one.kind === kind && made.includes(one.name)).sort((a, b) => (b.changed_at ?? "").localeCompare(a.changed_at ?? ""))[0];
        return row === undefined ? [] : [{ part: PART[kind], text: teachFlavour()?.mod?.(row)?.also ?? row.description ?? row.name, mod: { name: row.name, onOpen: () => askToOpenMod(row.name, kind) } }];
      }),
    [listed, made],
  );
  return lines.length === 0 ? null : <AlsoChanges lines={lines} />;
}

export function alsoChanges(entry: LogbookEntry): ReactNode {
  return <Also entry={entry} />;
}
