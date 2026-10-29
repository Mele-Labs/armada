// The tasks of his projects a Job holds, for its rail under the brief: each task or subtask whose holder is the Job, with the
// project it is in, or No project, and its state: held, or asking once the Job says it is done. A press opens it on the Projects
// tab as the palette does, through a window event, as `open-session.ts` is, because the rail does not hold the app's navigation.

import { useEffect, useMemo } from "react";

import type { ProjectRow, RowState, TaskRow } from "./model";
import { allRows, rowState, useProjects } from "./store";
import { NO_PROJECT } from "./views";

export type HeldTask = { id: string; title: string; project: string; state: Exclude<RowState, "closed"> };

/** The tasks the Job holds, loose ones first, then each project's in his order. None where no Projects store is mounted. */
export function useTasksHeld(sessionId: string): HeldTask[] {
  const { loose, projects } = useProjects();
  return useMemo(() => {
    const held = (rows: readonly ProjectRow[], project: string): HeldTask[] =>
      allRows({ rows })
        .filter((row): row is TaskRow => row.kind === "task" && row.job?.id === sessionId)
        .map((row) => ({ id: row.id, title: row.title, project, state: rowState(row) as HeldTask["state"] }));
    return [...held(loose, NO_PROJECT), ...projects.flatMap((one) => held(one.rows, one.name))];
  }, [loose, projects, sessionId]);
}

const SHOW_TASK = "armada:show-task";

export function askToShowTask(rowId: string): void {
  window.dispatchEvent(new CustomEvent<string>(SHOW_TASK, { detail: rowId }));
}

/** The app's way to a task on Projects, the palette's own; `undefined` where the window keeps no projects. */
export function useShowTaskAsked(show: ((rowId: string) => void) | undefined): void {
  useEffect(() => {
    if (show === undefined) return;
    const heard = (event: Event): void => show((event as CustomEvent<string>).detail);
    window.addEventListener(SHOW_TASK, heard);
    return () => window.removeEventListener(SHOW_TASK, heard);
  }, [show]);
}
