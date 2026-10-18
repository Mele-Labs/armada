// What Helm's thread links: a pull request or an issue by its number, a Session. **Each opens where
// Bridge already shows it**: a Session on the Cockpit, a pull request on its row of the merge
// line, and an issue in the Dispatch panel. The finding is `@armada/helm`'s `scanRefs`; this holds what
// it reads off the window and what each press does.
//
// A number the repository's open lists do not hold (a merged pull request) is looked up once, and kept: the same
// read `#` in the message box makes.

import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { ProseRefs } from "@armada/components";
import type { NumberedHit, ProseReference, ProseReferences } from "@armada/components";
import { branchOfPullRequest, scanRefs } from "@armada/helm";
import type { RefWorld } from "@armada/helm";
import type { RepositorySummary } from "@armada/protocol";

import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { useHelmSession } from "./helm-session";
import { useNumbered } from "./numbered";
import { openInPanel } from "./shared-panel/store";
import { useSessions } from "./sessions-draft";

export type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  /** Where each kind is shown. A pull request is its branch's row on the merge line. */
  openSession: (id: string) => void;
  openPullRequest: (branch: string | undefined) => void;
  openIssues: (root: string | undefined) => void;
  /** The forge, where a pull request or an issue has no place of its own here. */
  openLink: (address: string) => void;
};

/** Every number the thread's words hold, as `#N` or as the forge's address for one. */
function numbersIn(session: Session | undefined): number[] {
  const words = (session?.rows ?? []).flatMap((row) => (row.kind === "message" || row.kind === "sent" ? [row.text] : []));
  return words.flatMap((text) => [
    ...[...text.matchAll(/(?<![\w&/#])#(\d+)(?!\w)/g)].map((one) => Number(one[1])),
    ...[...text.matchAll(/https?:\/\/[^\s<>`)\]]+\/(?:pull|issues)\/(\d+)(?![\w/])/g)].map((one) => Number(one[1])),
  ]);
}

/** Where a reference is read: the repository it is in, the Session whose words and pull requests it holds, and a pull request that Session shows itself. */
type Scope = { manifestId: string | undefined; session: Session | undefined; showOwn?: (number: number) => void };

function useRefs({ state, repositories, openSession, openPullRequest, openIssues, openLink }: Host, { manifestId, session: thread, showOwn }: Scope): ProseReferences {
  const root = repositories.find((one) => one.manifest?.id === manifestId)?.root;
  const numbered = useNumbered(manifestId);
  const sessions = useSessions();
  const [looked, setLooked] = useState<Record<string, NumberedHit | null>>({});
  const wanted = numbersIn(thread).join();

  // A number the open lists do not hold is read once.
  useEffect(() => {
    if (numbered === undefined) return;
    const open = new Set(numbered.open.map((one) => one.tag.id));
    for (const number of new Set(wanted === "" ? [] : wanted.split(",").map(Number))) {
      const key = `${manifestId}#${number}`;
      if (open.has(String(number)) || key in looked) continue;
      setLooked((was) => ({ ...was, [key]: null }));
      void numbered.find(number).then((hit) => setLooked((was) => ({ ...was, [key]: hit ?? null })));
    }
  }, [wanted, numbered, manifestId, looked]);

  return useMemo((): ProseReferences => {
    // A pull request a Session opened is known by that Session, whatever the forge's open list says.
    const opened = (thread?.attachments ?? []).flatMap((one): NumberedHit[] =>
      one.kind === "pull_request" ? [{ tag: { kind: "pull_request", id: String(one.number), title: `#${one.number} ${one.title}`, url: one.address } }] : [],
    );
    const known = [...opened, ...(numbered?.open ?? []), ...Object.entries(looked).flatMap(([key, hit]) => (hit !== null && key.startsWith(`${manifestId}#`) ? [hit] : []))];
    const world: RefWorld = {
      // Helm's own Session is named by its repository, which would link every mention of it. An ended one
      // still has its page.
      sessions: sessions.filter((one) => one.helm !== true),
      numbered: known.flatMap((one) =>
        one.tag.kind === "issue" || one.tag.kind === "pull_request" ? [{ kind: one.tag.kind, number: one.tag.id, title: one.tag.title, ...(one.tag.url === undefined ? {} : { url: one.tag.url }) }] : [],
      ),
    };
    const line = state.mergeLines?.lines.find((one) => one.root === root);
    const addressOf = (ref: ProseReference) => world.numbered.find((one) => one.kind === ref.kind && one.number === ref.id)?.url;
    return {
      scan: (text) => scanRefs(text, world),
      open: (ref: ProseReference) => {
        if (ref.kind === "session") return openSession(ref.id);
        const address = addressOf(ref);
        // The side panel takes both a pull request and an issue, where a window hosts one.
        if (ref.kind === "pull_request" && openInPanel({ kind: "pr", number: Number(ref.id), address: address ?? "", ...(root === undefined ? {} : { root }) })) return;
        if (ref.kind === "issue" && openInPanel({ kind: "issue", number: Number(ref.id), ...(root === undefined ? {} : { root }), ...(manifestId === undefined ? {} : { manifestId }), ...(address === undefined ? {} : { address }) })) return;
        if (ref.kind === "pull_request") {
          const branch = branchOfPullRequest(line, ref.id);
          // Its Session's own panel, then its row on the merge line, then the forge: a merged one has no row.
          if (showOwn !== undefined && opened.some((one) => one.tag.id === ref.id)) showOwn(Number(ref.id));
          else if (branch !== undefined || address === undefined) openPullRequest(branch);
          else openLink(address);
        } else openIssues(root);
      },
    };
  }, [numbered, looked, manifestId, thread, state.mergeLines, sessions, root, showOwn, openSession, openPullRequest, openIssues, openLink]);
}

/** What Helm's thread links, read in its repository and from its own Session. */
export function useHelmRefs(host: Host): ProseReferences {
  const manifestId = host.state.helm.state === "none" ? undefined : host.state.helm.manifestId;
  return useRefs(host, { manifestId, session: useHelmSession(manifestId) });
}

/** The host's openers, for the Sessions screen's threads to link with: set once by `App`. */
export const RefsHost = createContext<Host | undefined>(undefined);

/** Words in one Session's thread linked as that Session's: in its own repository, and its pull requests in its own panel. */
export function SessionRefs({ session, show, children }: { session: Session; show: (number: number) => void; children: ReactNode }) {
  const host = useContext(RefsHost);
  if (host === undefined) return <>{children}</>;
  return <Linked host={host} session={session} show={show}>{children}</Linked>;
}

function Linked({ host, session, show, children }: { host: Host; session: Session; show: (number: number) => void; children: ReactNode }) {
  const refs = useRefs(host, { manifestId: session.manifestId, session, showOwn: show });
  return <ProseRefs.Provider value={refs}>{children}</ProseRefs.Provider>;
}
