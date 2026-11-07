// The mods a Mod Job made, from Fleet's mod list: the rows whose `made_by` is the Job. Each has the switch the Mods page has, and a mod
// applies as it is saved, so turning one off is the undo.

import { useMemo } from "react";
import type { ModKind, ModSummary } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { tabTitle } from "./job-panes/custom";
import { useModList } from "./mod-list";
import { askToTell } from "./tell";

export type ModMade = {
  name: string;
  title: string;
  kind: ModKind;
  enabled: boolean;
  changedAt?: string;
};

/** A mod's folder name as a person reads it, as the Mods page titles a theme: `dusk` is Dusk, `built-in:calm` is Calm. */
export const modTitle = (name: string): string => {
  const bare = name.startsWith("built-in:") ? name.slice("built-in:".length) : name;
  return bare.charAt(0).toUpperCase() + bare.slice(1);
};

/** A made mod's card title: a tab reads as its tab does, so the card and the tab never disagree. */
export const madeTitle = (name: string, kind: ModKind): string =>
  kind === "tab" ? tabTitle(name) : modTitle(name);

const madeBy =
  (jobId: string) =>
  (
    row: ModSummary,
  ): row is ModSummary & {
    kind: ModKind;
  } =>
    row.made_by === jobId && row.kind !== undefined;

/** Fleet's own `set_mod_enabled`, whose refusal is told as the Mods page tells it. */
export function setModEnabled(name: string, enabled: boolean): void {
  void window.armada.setModEnabled(name, enabled).then((outcome) => {
    if (!outcome.ok) askToTell(refusalWords(outcome));
  });
}

/** The Job's mods, and the switch for each: Fleet's own `set_mod_enabled`, whose refusal is told as the Mods page tells it. */
export function useModsMade(jobId: string): {
  mods: readonly ModMade[];
  setEnabled: (name: string, enabled: boolean) => void;
} {
  const listed = useModList();
  const mods = useMemo(
    () =>
      listed.filter(madeBy(jobId)).map((row) => ({
        name: row.name,
        title: madeTitle(row.name, row.kind),
        kind: row.kind,
        enabled: row.enabled,
        ...(row.changed_at === undefined
          ? {}
          : {
              changedAt: row.changed_at,
            }),
      })),
    [
      listed,
      jobId,
    ],
  );
  return {
    mods,
    setEnabled: setModEnabled,
  };
}
