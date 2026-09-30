// Routes Bridge sends that Fleet does not serve yet, and the issue that builds
// each one.
//
// **An entry says a control is on screen ahead of its route.** Where Fleet
// answers a command to one of these as a route it does not have, that is the
// route not being built yet rather than the two sides disagreeing, and Bridge
// raises `bridge.not_implemented` naming the issue — so the debug info a person
// copies is the brief an agent needs to build it. The error contract says so,
// under "What a person quotes".
//
// **Delete the entry when the route ships.** One left behind never fires: a
// served route answers, and its refusals carry a code, so neither reaches the
// match below. The path is Fleet's pattern with `{name}` for each segment the
// caller fills; Bridge builds the real path where it sends, never from here.

export type PendingRoute = {
  method: "GET" | "POST";
  /** Fleet's path, `{name}` for each segment the sender fills. */
  path: string;
  /** The act on screen that sends it, as the debug info names it. */
  act: string;
  /** The issue that builds the route. */
  issue: number;
};

export const PENDING_ROUTES: readonly PendingRoute[] = [
  { method: "POST", path: "/jobs/{job_id}/processes/{pid}/kill", act: "kill_process", issue: 1647 },
  { method: "POST", path: "/jobs/{job_id}/processes/kill", act: "kill_processes", issue: 1647 },
  { method: "POST", path: "/jobs/{job_id}/tasks/{task_id}/pilot", act: "pilot_task", issue: 250 },
  { method: "POST", path: "/jobs/{job_id}/tasks/{task_id}/restart", act: "restart_task", issue: 1656 },
  { method: "POST", path: "/jobs/{job_id}/tasks/{task_id}/edit", act: "edit_task", issue: 1657 },
  { method: "POST", path: "/jobs/{job_id}/plan/move", act: "move_plan", issue: 1685 },
];

/**
 * What Edit this task sends to `edit_task` (#1657). **Only the fields a person
 * changed.** Fleet has not agreed a body for the route yet, so this is what
 * Bridge sends and the debug info carries, not a wire type.
 */
export type EditTask = {
  title?: string;
  note?: string;
  scope?: string[];
  expects?: string;
  model?: string;
};

/**
 * What a drop on the plan sends to `move_plan` (#1685): a group to a new place
 * among the groups, or, with `task`, that task into `group` at `to`. `to`
 * counts from zero, in the order after the move. Fleet has not agreed a body
 * yet, so this is what Bridge sends and the debug info carries.
 */
export type MovePlan = {
  group: string;
  task?: string;
  to: number;
};

/**
 * A request body as the debug info carries it: each top-level field, a string
 * as itself and anything else as JSON. Anything that is not an object carries
 * nothing.
 */
export function sentOf(body: unknown): Record<string, string> {
  if (body === null || typeof body !== "object") return {};
  return Object.fromEntries(
    Object.entries(body).map(([key, value]) => [key, typeof value === "string" ? value : JSON.stringify(value)]),
  );
}

/** Where an issue number opens. The debug info carries the whole link, so a paste opens from anywhere. */
export function issueLink(issue: number): string {
  return `https://github.com/NickMele/armada/issues/${issue}`;
}

/** A pending route a request was sent to, and what the request filled each `{name}` with. */
export type PendingAsked = { route: PendingRoute; filled: Record<string, string> };

/**
 * The pending route `method` and `path` name, or `null`. `path` is what was
 * sent — ids encoded, maybe a query — so each segment is decoded before it is
 * handed back.
 */
export function pendingAt(method: string, path: string): PendingAsked | null {
  const sent = (path.split("?")[0] ?? "").split("/");
  for (const route of PENDING_ROUTES) {
    if (route.method !== method) continue;
    const pattern = route.path.split("/");
    if (pattern.length !== sent.length) continue;
    const filled: Record<string, string> = {};
    const matches = pattern.every((part, at) => {
      const segment = sent[at] ?? "";
      const name = /^\{(\w+)\}$/.exec(part)?.[1];
      if (name === undefined) return part === segment;
      filled[name] = decodeURIComponent(segment);
      return segment !== "";
    });
    if (matches) return { route, filled };
  }
  return null;
}
