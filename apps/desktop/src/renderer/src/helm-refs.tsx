// What Helm's thread links: a pull request or an issue by its number, a Session. **Each opens where
// Bridge already shows it**: a Session on the Cockpit, a pull request or an issue in the side panel;
// another repository's number is its forge page. The finding is `@armada/helm`'s `refScanner`; this holds what
// it reads off the window and what each press does.
//
// A number the repository's open lists do not hold (a merged pull request) is looked up once, and kept: the same
// read `#` in the message box makes. A path in a code span is asked of main once, and links only where it is a file in the Job's checkout.

import { createContext, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import { ProseRefs } from "@armada/components";
import type { NumberedHit, ProseReference, ProseReferences } from "@armada/components";
import { forgeAddress, forgeSlug, isElsewhere, refScanner } from "@armada/helm";
import type { RefWorld } from "@armada/helm";
import type { RepositorySummary } from "@armada/protocol";

import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { useHelmSession } from "./helm-session";
import { useNumbered } from "./numbered";
import { openInPanel } from "./shared-panel/store";
import { useSessions } from "./sessions-draft";
import { useTaughtLinks } from "./teach";
import { jobIdOf } from "@armada/shell";

export type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  /** Where a Session is shown. A pull request and an issue open in the side panel. */
  openSession: (id: string) => void;
  /** The forge, where another repository's number has no place of its own here. */
  openLink: (address: string) => void;
};

/** Every number the thread's words hold, as `#N`, as `owner/name#N` of this repository, or as the forge's address for one. */
function numbersIn(session: Session | undefined, slug: string | undefined): number[] {
  const words = (session?.rows ?? []).flatMap((row) => (row.kind === "message" || row.kind === "sent" ? [row.text] : []));
  return words.flatMap((text) => [
    ...[...text.matchAll(/(?<![\w&/#])#(\d+)(?!\w)/g)].map((one) => Number(one[1])),
    ...[...text.matchAll(/(?<![\w./-])([\w.-]+\/[\w.-]+)#(\d+)(?!\w)/g)].filter((one) => slug !== undefined && one[1]!.toLowerCase() === slug.toLowerCase()).map((one) => Number(one[2])),
    ...[...text.matchAll(/https?:\/\/[^\s<>`)\]]+\/(?:pull|issues)\/(\d+)(?![\w/])/g)].map((one) => Number(one[1])),
  ]);
}

/** A file's name as a model writes it in backticks: an extension, and folders before it if it has any. */
const PATH = /^\/?[\w.@+-]+(?:\/[\w.@+-]+)*\.[A-Za-z0-9]+$/;

/** Every inline code span of the thread's words that reads as a file path. */
function pathsIn(session: Session | undefined): string[] {
  const words = (session?.rows ?? []).flatMap((row) => (row.kind === "message" || row.kind === "sent" ? [row.text] : []));
  return [...new Set(words.flatMap((text) => [...text.matchAll(/`([^`\n]+)`/g)].map((one) => one[1]!).filter((one) => PATH.test(one))))];
}

/** Where a reference is read: the repository it is in, and the Session whose words and pull requests it holds. */
type Scope = { manifestId: string | undefined; session: Session | undefined };

function useRefs({ repositories, openSession, openLink }: Host, { manifestId, session: thread }: Scope): ProseReferences {
  const root = repositories.find((one) => jobIdOf(one) === manifestId)?.root;
  const numbered = useNumbered(manifestId);
  const sessions = useSessions();
  const [looked, setLooked] = useState<Record<string, NumberedHit | null>>({});
  // The repository's own `owner/name`, read off the addresses it already holds.
  const slug = forgeSlug([
    ...(thread?.attachments ?? []).flatMap((one) => (one.kind === "pull_request" ? [one.address] : [])),
    ...(numbered?.open ?? []).map((one) => one.tag.url),
    ...Object.entries(looked).flatMap(([key, one]) => (key.startsWith(`${manifestId}#`) ? [one?.tag.url] : [])),
  ]);
  const wanted = numbersIn(thread, slug).join();
  // Whether each path the thread writes is a file in the Job's checkout, by the Session it was asked for. A path is asked once.
  const sessionId = thread?.id;
  const [filed, setFiled] = useState<Record<string, boolean>>({});
  const paths = pathsIn(thread).join("\n");
  useEffect(() => {
    if (sessionId === undefined || paths === "") return;
    const fresh = paths.split("\n").filter((one) => !(`${sessionId}\n${one}` in filed));
    if (fresh.length === 0) return;
    setFiled((was) => ({ ...was, ...Object.fromEntries(fresh.map((one) => [`${sessionId}\n${one}`, false])) }));
    void window.armada.findPanelFiles(sessionId, fresh).then((there) => setFiled((was) => ({ ...was, ...Object.fromEntries(there.map((one) => [`${sessionId}\n${one}`, true])) })));
  }, [paths, sessionId, filed]);

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

  // A pull request a Session opened is known by that Session, whatever the forge's open list says.
  const opened = (thread?.attachments ?? []).flatMap((one): NumberedHit[] =>
    one.kind === "pull_request" ? [{ tag: { kind: "pull_request", id: String(one.number), title: `#${one.number} ${one.title}`, url: one.address } }] : [],
  );
  const known = [...opened, ...(numbered?.open ?? []), ...Object.entries(looked).flatMap(([key, hit]) => (hit !== null && key.startsWith(`${manifestId}#`) ? [hit] : []))];
  // Words the owner taught as links, in this repository or in every one, and the rule the Teach panel is showing back.
  const taught = useTaughtLinks(manifestId, root?.split("/").pop());
  const linkable: RefWorld = {
    ...(slug === undefined ? {} : { slug }),
    ...(taught.length === 0 ? {} : { links: taught }),
    files: Object.entries(filed).flatMap(([key, there]) => (there && sessionId !== undefined && key.startsWith(`${sessionId}\n`) ? [key.slice(sessionId.length + 1)] : [])),
    // Helm's own Session is named by its repository, which would link every mention of it. An ended one
    // still has its page.
    sessions: sessions.flatMap((one) => (one.helm === true ? [] : [{ id: one.id, ...(one.title === undefined ? {} : { title: one.title }) }])),
    numbered: known.flatMap((one) =>
      one.tag.kind === "issue" || one.tag.kind === "pull_request" ? [{ kind: one.tag.kind, number: one.tag.id, title: one.tag.title, ...(one.tag.url === undefined ? {} : { url: one.tag.url }) }] : [],
    ),
  };
  // **The world is held on what it can link**, not on the list of Sessions or the thread: any Session's message makes
  // those new, and a new world scans every message in the thread again.
  const worldKey = JSON.stringify(linkable);
  const world = useMemo(() => linkable, [worldKey]);
  const scan = useMemo(() => refScanner(world), [world]);
  // The openers are new on each render of `App`; a press reads the latest, so they are not what the links are held on.
  const openers = useRef({ openSession, openLink });
  openers.current = { openSession, openLink };

  return useMemo((): ProseReferences => {
    const addressOf = (ref: ProseReference) => world.numbered.find((one) => one.kind === ref.kind && one.number === ref.id)?.url;
    return {
      scan,
      open: (ref: ProseReference) => {
        if (ref.kind === "session") return openers.current.openSession(ref.id);
        // A taught link's id is its address, which opens in the browser as any web address does.
        if (ref.kind === "link") return openers.current.openLink(ref.id);
        if (ref.kind === "file") return openInPanel({ kind: "file", path: ref.id, from: "session", ...(sessionId === undefined ? {} : { sessionId }) });
        // Another repository's number is only a page on the forge.
        if (isElsewhere(ref.id)) return openers.current.openLink(forgeAddress(ref));
        const address = addressOf(ref);
        if (ref.kind === "pull_request") openInPanel({ kind: "pr", number: Number(ref.id), address: address ?? "", ...(root === undefined ? {} : { root }) });
        else openInPanel({ kind: "issue", number: ref.id, ...(root === undefined ? {} : { root }), ...(manifestId === undefined ? {} : { manifestId }), ...(address === undefined ? {} : { address }) });
      },
    };
  }, [world, scan, manifestId, root, sessionId]);
}

/** What Helm's thread links, read in its repository and from its own Session. */
export function useHelmRefs(host: Host): ProseReferences {
  const manifestId = host.state.helm.state === "none" ? undefined : host.state.helm.manifestId;
  return useRefs(host, { manifestId, session: useHelmSession(manifestId) });
}

/** The host's openers, for the Sessions screen's threads to link with: set once by `App`. */
export const RefsHost = createContext<Host | undefined>(undefined);

/** Words in one Session's thread linked as that Session's: in its own repository, and its pull requests in the side panel. */
export function SessionRefs({ session, children }: { session: Session; children: ReactNode }) {
  const host = useContext(RefsHost);
  if (host === undefined) return <>{children}</>;
  return <Linked host={host} session={session}>{children}</Linked>;
}

function Linked({ host, session, children }: { host: Host; session: Session; children: ReactNode }) {
  const refs = useRefs(host, { manifestId: session.manifestId, session });
  return <ProseRefs.Provider value={refs}>{children}</ProseRefs.Provider>;
}
