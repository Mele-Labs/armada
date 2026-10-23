// What a taught Logbook entry also changes, for the line under its text: a line for each mod the teaching made, Words or Fleet. Fleet's
// mod list names the entry each mod came from; a Fleet mod's line is worded from the file Fleet checked for it. Forget on the entry has
// Fleet remove its mods, and with them every line here.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { AlsoChanges } from "@armada/components";
import type { AlsoChangesProps } from "@armada/components";
import type { LogbookEntry } from "@armada/protocol";

import { useModList } from "./mod-list";
import { askToOpenMod } from "./open-mod";
import { fleetAlso } from "./taught-parts";

type Line = AlsoChangesProps["lines"][number];

const PART = { words: "Words", fleet: "Fleet" } as const;

function Also({ entry }: { entry: LogbookEntry }) {
  const listed = useModList();
  // A teaching makes at most one mod of a kind, so the line is the newest listed.
  const rows = useMemo(
    () =>
      (["words", "fleet"] as const).flatMap((kind) => {
        const row = listed.filter((one) => one.kind === kind && one.entry === entry.id).sort((a, b) => (b.changed_at ?? "").localeCompare(a.changed_at ?? ""))[0];
        return row === undefined ? [] : [{ kind, row }];
      }),
    [listed, entry.id],
  );
  // What each Fleet mod does, from its checked file, for the change of it read.
  const [said, setSaid] = useState<Readonly<Record<string, { at: string; text: string | undefined }>>>({});
  useEffect(() => {
    let live = true;
    for (const { kind, row } of rows) {
      const at = row.changed_at ?? "";
      if (kind !== "fleet" || said[row.name]?.at === at) continue;
      void window.armada.validateMod(row.name).then((checked) => {
        if (live && checked?.valid) setSaid((was) => ({ ...was, [row.name]: { at, text: checked.fleet === undefined ? undefined : fleetAlso(checked.fleet) } }));
      });
    }
    return () => {
      live = false;
    };
  }, [rows, said]);
  const lines = rows.map(({ kind, row }): Line => {
    const read = said[row.name];
    const worded = kind === "fleet" && read?.at === (row.changed_at ?? "") ? read.text : undefined;
    return { part: PART[kind], text: worded ?? row.description ?? row.name, mod: { name: row.name, onOpen: () => askToOpenMod(row.name, kind) } };
  });
  return lines.length === 0 ? null : <AlsoChanges lines={lines} />;
}

export function alsoChanges(entry: LogbookEntry): ReactNode {
  return <Also entry={entry} />;
}
