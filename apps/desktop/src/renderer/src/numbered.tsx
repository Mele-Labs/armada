// What `#` offers in a message box: a repository's open issues and open pull requests, by number,
// and one number looked up when it is not among them.
//
// **Held here and not read, because no Fleet call serves Bridge this yet.** A real Fleet answers the
// open issues (`list_issues`), the open pull requests (the merge line's `hub.pull_requests`) and one
// pull request by number (`get_pull_request`), but only the first has a capability in the preload;
// the Sessions seam has no `issue` tag kind either (`TagKind` in `crates/ipc/src/hosted_sessions.rs`).
// A mock scenario hands this down, the app's own mount provides none, and `#` is then only a character.
// The tag helpers are the real mapping: an issue and a pull request are tagged as `#214 The title`.

import { createContext, useContext, useMemo } from "react";
import type { ReactNode } from "react";

import type { ComposerNumbered, NumberedHit } from "@armada/components";
import type { HubPullRequest, PullRequestState, TrackedIssue } from "@armada/protocol";

/** An open issue, as `#` tags it. */
export const issueHit = (issue: Pick<TrackedIssue, "number" | "title">): NumberedHit => ({
  tag: { kind: "issue", id: String(issue.number), title: `#${issue.number} ${issue.title}` },
});

/** An open pull request off the merge line's hub. */
export const pullRequestHit = (pull: Pick<HubPullRequest, "number" | "title">): NumberedHit => ({
  tag: { kind: "pull_request", id: String(pull.number), title: `#${pull.number} ${pull.title}` },
});

/** A pull request `get_pull_request` answered, whatever its standing: one that is not open says so. */
export function lookedUpPullRequest(pull: Pick<PullRequestState, "number" | "title" | "state">): NumberedHit {
  const { tag } = pullRequestHit(pull);
  return pull.state === "merged" ? { tag, standing: "Merged" } : pull.state === "closed" ? { tag, standing: "Closed" } : { tag };
}

/** One source for every repository, read by its Manifest id. */
export type NumberedSource = {
  open: (manifestId: string | undefined) => readonly NumberedHit[];
  find: (manifestId: string | undefined, number: number) => Promise<NumberedHit | undefined>;
};

const Held = createContext<NumberedSource | undefined>(undefined);

/** Hand a moment's issues and pull requests to the window. The mock's own mount is the one caller. */
export function NumberedFrom({ held, children }: { held: NumberedSource | undefined; children: ReactNode }) {
  return <Held.Provider value={held}>{children}</Held.Provider>;
}

/** What `#` reads for one repository, or `undefined` where nothing serves it. */
export function useNumbered(manifestId: string | undefined): ComposerNumbered | undefined {
  const source = useContext(Held);
  return useMemo(
    () => (source === undefined ? undefined : { open: source.open(manifestId), find: (number: number) => source.find(manifestId, number) }),
    [source, manifestId],
  );
}
