// What `#` offers in a message box: a repository's open issues and open pull requests, by number,
// and one number looked up when it is not among them.
//
// **Two sources fill it.** The app's own mount reads a real Fleet (`numbered-fleet.tsx`): `list_issues`,
// the merge line's `hub.pull_requests`, and `get_pull_request` then `get_issue` for a number that is not
// open. A mock scenario holds the same shapes (`mock/numbered.ts`). The hit helpers below are the one
// mapping: an issue and a pull request are tagged as `#2214 The title`, with the forge's address.

import { createContext, useContext, useMemo, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { ComposerNumbered, NumberedHit } from "@armada/components";
import type { HubPullRequest, PullRequestState, TrackedIssue } from "@armada/protocol";

/**
 * An open issue, as `#` tags it. **The tag's id is the number on a forge, and the key where the tracker names
 * issues by one** (`ABC-123`), and its title leads with the reference either way.
 */
export const issueHit = (issue: Pick<TrackedIssue, "reference" | "title"> & { url?: string }): NumberedHit => {
  const number = /^#(\d+)$/.exec(issue.reference)?.[1];
  return { tag: { kind: "issue", id: number ?? issue.reference, title: `${issue.reference} ${issue.title}`, ...(issue.url === undefined ? {} : { url: issue.url }) } };
};

/** An open pull request off the merge line's hub. */
export const pullRequestHit = (pull: Pick<HubPullRequest, "number" | "title"> & { url?: string }): NumberedHit => ({
  tag: { kind: "pull_request", id: String(pull.number), title: `#${pull.number} ${pull.title}`, ...(pull.url === undefined ? {} : { url: pull.url }) },
});

/** A pull request `get_pull_request` answered, whatever its standing: one that is not open says so. */
export function lookedUpPullRequest(pull: Pick<PullRequestState, "number" | "title" | "state" | "address">): NumberedHit {
  const { tag } = pullRequestHit({ number: pull.number, title: pull.title, url: pull.address });
  return pull.state === "merged" ? { tag, standing: "Merged" } : pull.state === "closed" ? { tag, standing: "Closed" } : { tag };
}

/** One source for every repository, read by its Manifest id. */
export type NumberedSource = {
  open: (manifestId: string | undefined) => readonly NumberedHit[];
  find: (manifestId: string | undefined, number: number) => Promise<NumberedHit | undefined>;
  /** A source that reads over time says when what it holds changed. One that holds fixtures leaves both out. */
  subscribe?: (listener: () => void) => () => void;
  /** Moves each time `open` would answer something new. */
  version?: () => number;
};

const Held = createContext<NumberedSource | undefined>(undefined);

/** Hand the window its issues and pull requests: a real Fleet's from the app's mount, a moment's from the mock's. */
export function NumberedFrom({ held, children }: { held: NumberedSource | undefined; children: ReactNode }) {
  return <Held.Provider value={held}>{children}</Held.Provider>;
}

/** What `#` reads for one repository, or `undefined` where nothing serves it. */
export function useNumbered(manifestId: string | undefined): ComposerNumbered | undefined {
  const source = useContext(Held);
  const version = useSyncExternalStore(source?.subscribe ?? NEVER, source?.version ?? NONE_YET);
  // `open` is read where `#` is, each time, so a source may refresh a list it finds stale then. `version` is
  // not read here: it is why this is a new object, and the box draws again.
  return useMemo(
    (): ComposerNumbered | undefined => {
      if (source === undefined) return undefined;
      // Fixtures are read once; a source that reads over time is asked each time.
      const fixed = source.version === undefined ? source.open(manifestId) : undefined;
      return {
        get open() {
          return fixed ?? source.open(manifestId);
        },
        find: (number: number) => source.find(manifestId, number),
      };
    },
    [source, manifestId, version],
  );
}

const NEVER = (): (() => void) => () => undefined;
const NONE_YET = (): number => 0;
