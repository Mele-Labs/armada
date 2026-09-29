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
];

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
