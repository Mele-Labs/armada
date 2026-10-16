// What the list's search finds. A Session by its title, its id, a pull request number, a branch, a slot or a Job it holds, as the
// Sessions page found it (`sessionsMatching`); a Job by its title, id, handle or branch. A blank search finds nothing in particular
// and narrows nothing. When what matched is a thing the Session holds, the row says which, in place of its age.

import { useMemo } from "react";
import type { JobSummary } from "@armada/protocol";
import { sessionsMatching, type Session } from "@armada/screens/src/draft/sessions";

import { useSessions } from "../../sessions-draft";

/** What a row says about why it matched: short in the row, whole in the tooltip. */
export type Found = { short: string; whole: string };

const NONE: Found = { short: "", whole: "" };

function sessionFound(hit: ReturnType<typeof sessionsMatching>[number]): Found {
  const { matched } = hit;
  if (matched === "title" || matched === "id") return NONE;
  switch (matched.kind) {
    case "pull_request":
      return { short: `#${matched.number}`, whole: `Pull request #${matched.number}` };
    case "branch":
      return { short: matched.name, whole: `Branch ${matched.name}` };
    case "slot":
      return { short: `slot ${matched.slot}`, whole: `Worktree slot ${matched.slot}` };
    case "job":
      return { short: `job ${matched.number}`, whole: `Job ${matched.number}, ${matched.title}` };
    default:
      return NONE;
  }
}

/** The rows a search finds, by their list key, or `undefined` for a blank search. */
export function foundOf(query: string, sessions: readonly Session[], jobs: readonly JobSummary[]): ReadonlyMap<string, Found> | undefined {
  const said = query.trim().toLowerCase();
  if (said === "") return undefined;
  const found = new Map<string, Found>();
  for (const hit of sessionsMatching(sessions, query)) found.set(`session:${hit.session.id}`, sessionFound(hit));
  const word = said.replace(/^(job|#)\s*/, "");
  for (const job of jobs) {
    if (job.title.toLowerCase().includes(said)) found.set(`job:${job.id}`, NONE);
    else if (job.id === word || job.handle.toLowerCase().includes(word)) found.set(`job:${job.id}`, { short: job.handle, whole: `Job ${job.handle}` });
    else if (job.branch?.toLowerCase().includes(said) === true) found.set(`job:${job.id}`, { short: job.branch!, whole: `Branch ${job.branch}` });
  }
  return found;
}

/** The search, read against the Sessions and Jobs the window holds. */
export function useFound(query: string, jobs: readonly JobSummary[]): ReadonlyMap<string, Found> | undefined {
  const sessions = useSessions();
  return useMemo(() => foundOf(query, sessions, jobs), [query, sessions, jobs]);
}
