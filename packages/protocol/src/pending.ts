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
  { method: "POST", path: "/jobs/{job_id}/tasks/{task_id}/pilot", act: "pilot_task", issue: 250 },
  { method: "POST", path: "/jobs/{job_id}/approve_wave", act: "approve_wave", issue: 1694 },
  { method: "POST", path: "/jobs/{job_id}/edit", act: "edit_job", issue: 1699 },
];

/**
 * What Approve the plan sends to `approve_wave` (#1694) at an Epic Job's plan
 * gate: every Job of the proposed wave, each at `awaiting_approval` and
 * dispatched by the Epic, released together. Fleet has not agreed a body yet,
 * so this is what Bridge sends and the debug info carries.
 */
export type ApproveWave = {
  jobs: readonly string[];
};

/**
 * What Edit this Job sends to `edit_job` (#1699), on a Job of an Epic's
 * proposed wave, still at `awaiting_approval`. **Only the fields a person
 * changed**; `expects` is the whole list, one line each. Fleet has not agreed
 * a body yet, so this is what Bridge sends and the debug info carries.
 */
export type EditJob = {
  title?: string;
  brief?: string;
  expects?: string[];
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
