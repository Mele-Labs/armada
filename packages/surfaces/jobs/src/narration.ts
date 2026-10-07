// Which plan task a moment or a path belongs to — the placement rules
// `task-files.ts` reads to give each edit its task. #1185.
//
// **A window places a row. Where no window covers an Edit, the task whose
// `scope` names the file does** — #1498: a task marked `done` without ever
// being marked `working` has no window, so its own declared files read as
// changed outside every task. A path written down in `scope` is a declaration
// and reading it is not guessing; a task's title and its prose are still never
// matched.
import type { PlanTask } from "@armada/protocol";

import { instant } from "@armada/screens/src/duration";

/**
 * The task a turn at `ts` belongs to, by id.
 *
 * **Two tasks marked working at once overlap, and the one entered last wins** —
 * it is the one the Drone turned to most recently.
 */
export function taskAt(ts: string, tasks: readonly PlanTask[]): string | undefined {
  const at = instant(ts);
  if (at === null) return undefined;
  let best: { id: string; entered: number } | undefined;
  for (const task of tasks) {
    for (const window of task.working_windows ?? []) {
      const entered = instant(window.entered);
      const left = window.left === undefined ? null : instant(window.left);
      if (entered === null || at < entered || (left !== null && at >= left)) continue;
      if (best === undefined || entered >= best.entered) best = { id: task.id, entered };
    }
  }
  return best?.id;
}

/**
 * The task whose `scope` names `path`, by id — what places an edit no window
 * covers. Nothing here reads the clock, so a window always answers first.
 *
 * **Matched at a path segment, not against the diff.** A call names the file
 * under a worktree and a declaration is repository-relative, and `editsIn` is
 * never handed the diff to reconcile the two — `repoPathOf`'s rule, applied
 * without it. A declared directory holds the files under it, because
 * `declare_scope` takes both.
 *
 * **The more specific declaration wins, and a tie goes to plan order.** An
 * exact name beats a directory holding it, and a deeper directory beats a
 * shallower one.
 */
export function declaredBy(path: string, tasks: readonly PlanTask[]): string | undefined {
  let best: { id: string; exact: boolean; length: number } | undefined;
  for (const task of tasks) {
    for (const declared of task.scope ?? []) {
      const named = declared.endsWith("/") ? declared.slice(0, -1) : declared;
      if (named === "") continue;
      const exact = path === named || path.endsWith(`/${named}`);
      const under = path.startsWith(`${named}/`) || path.includes(`/${named}/`);
      if (!exact && !under) continue;
      const better =
        best === undefined ||
        (exact && !best.exact) ||
        (exact === best.exact && named.length > best.length);
      if (better) best = { id: task.id, exact, length: named.length };
    }
  }
  return best?.id;
}
