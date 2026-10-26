// The confirm a run asks first, and the key that presses Run: ⌥R runs the Job open on the stage on its bay, else the repository of the
// row the cursor is on, else the one picked. Mounted once by the deck, where the list and a Job's header press the same acts.

import { useEffect, useState } from "react";
import { RunConfirm } from "@armada/components";
import type { RunDraft } from "@armada/components";
import { repositoryLabel } from "@armada/shell";

import { useDispatching } from "../../dispatch-panel/dispatching";
import { useSessions } from "../../sessions-draft";
import type { CockpitProps } from "../deck/props";
import type { Ship } from "../deck/ship";
import { deckNow, openRun } from "../deck/store";
import { dismissAsked, pressRun, useAsked } from "./acts";
import { useRuns, useRunsState, type RunTarget } from "./source";
import { jobTarget } from "./target";

const BLANK: RunDraft["processes"] = [{ id: "new-1", name: "", folder: ".", command: "" }];

export function RunHost({ ship, props }: { ship: Ship; props: CockpitProps }) {
  const source = useRuns();
  const state = useRunsState();
  const asked = useAsked();
  const sessions = useSessions();
  const dispatching = useDispatching();
  const repositories = props.state.holds.repositories ?? [];
  // The draft Fleet answered for the confirm that is open, held against it so a later ask never shows an earlier draft.
  const [drafted, setDrafted] = useState<{ for: typeof asked; draft: RunDraft; run?: { id: string; name: string } }>();
  useEffect(() => {
    if (source === undefined || asked === undefined) return;
    let live = true;
    // An edit is drafted from the run saved, a first run from what Fleet reads of the repository.
    const reading =
      asked.edit === undefined
        ? source.draft(asked.target.repository).then((draft) => (draft === undefined ? undefined : { draft }))
        : source.saved(asked.edit).then((found) => (found === undefined ? undefined : { draft: found.draft, run: { id: found.run, name: found.name } }));
    void reading.then((found) => {
      if (!live) return;
      if (found === undefined) dismissAsked();
      else setDrafted({ for: asked, ...found });
    });
    return () => {
      live = false;
    };
  }, [source, asked]);
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
  if (source === undefined || asked === undefined || drafted?.for !== asked) return null;
  const draft: RunDraft = asked.add ? { ...drafted.draft, processes: BLANK } : drafted.draft;
  return (
    <RunConfirm
      open
      repository={asked.label}
      draft={draft}
      keep={`${asked.target.repository}:${drafted.run === undefined ? (asked.add ? "add" : "first") : `edit:${drafted.run.id}`}`}
      {...(drafted.run !== undefined ? { name: drafted.run.name, edit: true } : asked.add ? { name: "" } : {})}
      onCancel={dismissAsked}
      onFix={(brief) => {
        // What was typed in the confirm is kept, so it is there when the run is asked for again.
        dismissAsked();
        dispatching?.choose("session");
        dispatching?.openSheet(brief, asked.target.repository);
      }}
      onSave={(processes, name, setup) => {
        dismissAsked();
        void source.save(asked.target, asked.add || drafted.run !== undefined ? name : asked.label, processes, setup, drafted.run?.id).then((id) => {
          if (id !== undefined) openRun(id);
        });
      }}
    />
  );
}
