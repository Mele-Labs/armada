// What pressing Run does: start the run chosen and open its panel, or open the panel on a draft where the repository has none saved. The
// run being edited is held here and drawn by `RunPane`, so the list's heading, a Job's header and the panel's own Edit press one thing.

import { useSyncExternalStore } from "react";
import type { RunDraft, SavedRun } from "@armada/components";

import { openRun } from "../deck/store";
import type { RunsSource, RunTarget } from "./source";

/** The pane a run not saved yet is edited in, before it has an app of its own. */
export const DRAFT_PANE = "draft";

const BLANK: RunDraft["processes"] = [{ id: "new-1", name: "", folder: ".", command: "" }];

/**
 * The run being edited: what it runs, the draft and name it starts from, and what its typed fields are kept under. `add` is a run beside those
 * already saved; `edit` is the app whose saved run it is, and that run's id.
 */
export type Asked = { target: RunTarget; label: string; add: boolean; draft: RunDraft; name: string; keep: string; edit?: { app: string; run: string } };

let asked: Asked | undefined;
let asking = 0;
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const ask = (next: Asked | undefined) => {
  asked = next;
  listeners.forEach((one) => one());
};

export const useAsked = (): Asked | undefined => useSyncExternalStore(subscribe, () => asked);
export const dismissAsked = () => ask(undefined);

/** Whether `pane` is where `one` is edited: the draft's own pane, or the app whose run it is. */
export const askedIn = (one: Asked | undefined, pane: string): boolean => one !== undefined && (one.edit === undefined ? pane === DRAFT_PANE : one.edit.app === pane);

/** The edit drawn in `pane` is let go, as leaving the pane does. What was typed stays kept. */
export const letGo = (pane: string) => {
  if (askedIn(asked, pane)) dismissAsked();
};

/** Reads what the edit starts from, then opens it: a first run and an added one on the draft's pane, an edit on its app's own. A later press wins over an earlier read. */
async function edit(source: RunsSource, target: RunTarget, label: string, add: boolean, app?: string): Promise<void> {
  asking += 1;
  const mine = asking;
  const found: { draft: RunDraft; name: string; edit?: Asked["edit"] } | undefined =
    app === undefined
      ? await source.draft(target.repository).then((draft) => (draft === undefined ? undefined : { draft: add ? { ...draft, processes: BLANK } : draft, name: add ? "" : label }))
      : await source.saved(app).then((saved) => (saved === undefined ? undefined : { draft: saved.draft, name: saved.name, edit: { app, run: saved.run } }));
  if (mine !== asking || found === undefined) return;
  const keep = `${target.repository}:${found.edit === undefined ? (add ? "add" : "first") : `edit:${found.edit.run}`}`;
  ask({ target, label, add, keep, ...found });
  if (found.edit === undefined) openRun(DRAFT_PANE);
}

/** Run, as the face presses it (`run` is the last used, or none where nothing is saved) and as a row of the picker does. */
export function pressRun(source: RunsSource, target: RunTarget, label: string, run: SavedRun | undefined): void {
  if (run === undefined) void edit(source, target, label, false);
  else
    void source.start(target, run).then((id) => {
      if (id !== undefined) openRun(id);
    });
}

/** Add a way to run: the panel on a blank draft, over a repository that already has some. */
export const pressAddRun = (source: RunsSource, target: RunTarget, label: string): void => void edit(source, target, label, true);

/** Edit the run: the panel of the app it was started from, its rows turned into fields. */
export const pressEditRun = (source: RunsSource, target: RunTarget, label: string, app: string): void => void edit(source, target, label, false, app);
