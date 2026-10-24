// The mods a Mod Job made, from Fleet's mod list: the rows whose `made_by` is the Job. Each has the switch the Mods page has, and a mod
// applies as it is saved, so turning one off is the undo.

import { useCallback, useMemo } from "react";
import type { ModKind, ModSummary } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { useModList } from "./mod-list";
import { askToTell } from "./tell";

export type ModMade = { name: string; kind: ModKind; enabled: boolean };

const madeBy = (jobId: string) => (row: ModSummary): row is ModSummary & { kind: ModKind } => row.made_by === jobId && row.kind !== undefined;

/** The Job's mods, and the switch for each: Fleet's own `set_mod_enabled`, whose refusal is told as the Mods page tells it. */
export function useModsMade(jobId: string): { mods: readonly ModMade[]; setEnabled: (name: string, enabled: boolean) => void } {
  const listed = useModList();
  const mods = useMemo(() => listed.filter(madeBy(jobId)).map((row) => ({ name: row.name, kind: row.kind, enabled: row.enabled })), [listed, jobId]);
  const setEnabled = useCallback((name: string, enabled: boolean): void => {
    void window.armada.setModEnabled(name, enabled).then((outcome) => {
      if (!outcome.ok) askToTell(refusalWords(outcome));
    });
  }, []);
  return { mods, setEnabled };
}
