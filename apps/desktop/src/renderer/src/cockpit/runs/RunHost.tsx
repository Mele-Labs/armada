// The key that presses Run: ⌥R runs the Job open on the stage on its bay, else the repository of the row the cursor is on, else the one
// picked. Mounted once by the deck. A repository with no saved run opens the running panel on its draft (`acts.ts`).

import { useEffect } from "react";
import { repositoryLabel } from "@armada/shell";

import { useSessions } from "../../sessions-draft";
import type { CockpitProps } from "../deck/props";
import type { Ship } from "../deck/ship";
import { deckNow } from "../deck/store";
import { pressRun } from "./acts";
import { useRuns, useRunsState, type RunTarget } from "./source";
import { jobTarget } from "./target";

export function RunHost({ ship, props }: { ship: Ship; props: CockpitProps }) {
  const source = useRuns();
  const state = useRunsState();
  const sessions = useSessions();
  const repositories = props.state.holds.repositories ?? [];
  useEffect(() => {
    if (source === undefined) return;
    const onKey = (event: KeyboardEvent) => {
      const deck = deckNow();
      if (!deck.here || !event.altKey || event.metaKey || event.ctrlKey || event.shiftKey || event.code !== "KeyR") return;
      event.preventDefault();
      const open = deck.sel?.kind === "session" ? sessions.find((one) => `session:${one.id}` === deck.sel?.key) : undefined;
      const job = open === undefined ? undefined : jobTarget(open, repositories);
      const row = deck.cursor === undefined ? undefined : ship.subjects.find((one) => one.key === deck.cursor);
      const root = row?.root ?? props.picked?.root;
      const target: RunTarget | undefined = job ?? (root === undefined ? undefined : { repository: root, checkout: { kind: "main", branch: "main" } });
      if (target === undefined) return;
      const saved = state.saved[target.repository];
      const label = repositoryLabel(repositories.find((one) => one.root === target.repository) ?? { root: target.repository, records_root: "" }, repositories);
      const last = saved?.runs.find((one) => one.id === saved.last) ?? saved?.runs[0];
      pressRun(source, target, label, last);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [source, state, sessions, repositories, ship, props.picked]);
  return null;
}
