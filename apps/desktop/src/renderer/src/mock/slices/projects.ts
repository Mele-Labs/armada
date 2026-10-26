// The Projects tab, mock only. A scenario hands each repository's projects (`MockDraft.projectsTab`), built into a store per
// window. **A done ask is one question in two places**: on the row, and while the Job that held it is live, as a Needs you
// item on that Job. Answering either takes the other off, so the window's two stores are joined here, as Fleet would join
// them: the scenario hands its Sessions store over, and mounting hands the Projects store's answer back.

import type { Project } from "../../projects-tab/model";
import type { SessionsStore } from "../sessions/script";

export type ProjectsSeed = readonly Project[];

/** The Needs you item a finished Job carries for a task it held, by the row's id. */
export const doneAskId = (rowId: string): string => `task-done:${rowId}`;
export const rowOfAsk = (itemId: string): string | undefined => (itemId.startsWith("task-done:") ? itemId.slice("task-done:".length) : undefined);

/** This window's halves of the done ask. Set again for each window mounted. */
export const projectsLink: { jobs?: SessionsStore; rows?: (rowId: string, done: boolean) => void } = {};

/** A row answered on the Projects tab: its item is taken off the Job. */
export function settledOnRow(job: string, rowId: string): void {
  projectsLink.jobs?.edit(job, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== doneAskId(rowId)) }));
}
