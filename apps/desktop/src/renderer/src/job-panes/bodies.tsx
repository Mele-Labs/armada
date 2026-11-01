// The bodies of the tabs that stand on their own. A tab that needs the Job's thread or its story is drawn by `SessionWorkspace`, which holds both.

import { useEffect, useMemo, useRef, useState } from "react";
import { File, Folder, FolderOpen } from "lucide-react";
import { Select, UnifiedDiff, PlanRevisionView, useKept, useUnchanged, type LedgerEntry, type PlanRevision, ShellPanelPane } from "@armada/components";
import { attachmentsOf, linkedOf, type Session } from "@armada/screens/src/draft/sessions";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { useJobBrowser } from "../job-browser/store";
import { shellsOf, useJobShell } from "../job-shell";
import { useDeckShell } from "../job-shell-dock";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { RequestsPanel } from "../shared-panel/requests";
import { InlinePanel } from "../shared-panel/side";
import { openInPanel } from "../shared-panel/store";
import { IssueRow } from "../session-view/IssuesBlock";
import { PullRow } from "../session-view/LedgerRow";
import { Groups } from "../session-view/SessionLayers";
import { artifactsOf, linkedSessionsOf } from "../session-view/artifacts";
import { IssueJobCard } from "../issues-tab/marks";
import { MOCK_PATCH, MOCK_PATCH_LOG, MOCK_PATCH_TOOLTIP } from "./mock-diff";
import { jobOpens, useJobPanes } from "./store";
import { hasKind } from "./tree";

export function BrowserBody({ jobId }: { jobId: string }) {
  const browser = useJobBrowser(jobId);
  return browser === undefined ? null : <JobBrowserPane sessionId={jobId} browser={browser} placed="pane" />;
}

/** The Job's shell. The Cockpit's Terminal tile shows the same shell, which is why it asks for it shown while a Terminal tab is open. */
export function TerminalBody({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const served = draft?.terminalRuns !== undefined;
  const pane = useDeckShell(jobId, served);
  const shells = served ? shellsOf(window.armada) : null;
  const { shown } = useJobShell(shells, "page", jobId);
  useEffect(() => {
    if (shells !== null && !shown) shells.want("page", jobId);
    // Once, as the tab opens: a shell put away from its own head stays away.
  }, []);
  // The tab names the shell and the pane places it, so the head keeps only what the shell itself can do.
  return pane === null ? null : (
    <ShellPanelPane phase={pane.phase} onEnd={pane.onEnd} onOpen={pane.onOpen}>
      {pane.children}
    </ShellPanelPane>
  );
}

const cutSaid = (drawn: number, total: number) => `The first ${drawn.toLocaleString()} of ${total.toLocaleString()} lines.`;

/** Where changes can be read from: the Job's own branch, each pull request it opened, and each Job it started. */
type Source = { value: string; label: string; patch: string };

function sourcesOf(session: Session, all: readonly Session[]): Source[] {
  const patches = [MOCK_PATCH_TOOLTIP, MOCK_PATCH_LOG];
  return [
    { value: "branch", label: "Branch", patch: MOCK_PATCH },
    ...attachmentsOf(session, "pull_request").map((one, at): Source => ({ value: `pr:${one.number}`, label: `#${one.number} ${one.title}`, patch: at === 0 ? MOCK_PATCH : patches[0]! })),
    ...linkedOf(all, session).map(({ session: sub }): Source => {
      const pull = attachmentsOf(sub, "pull_request")[0];
      const title = sub.title ?? sub.id;
      return { value: `job:${sub.id}`, label: pull === undefined ? title : `${title} #${pull.number}`, patch: patches[1]! };
    }),
  ];
}

/** The Job's changes, from the place the picker names. The mock's patches are invented. */
export function ChangesBody({ session }: { session: Session }) {
  const sources = sourcesOf(session, useSessions());
  const [chosen, choose] = useKept(`changes:${session.id}`, "branch");
  const source = sources.find((one) => one.value === chosen) ?? sources[0]!;
  const drawn = drawnOf(source.patch, cutSaid);
  return (
    <>
      <div className="job-pane__pick">
        <Select aria-label="Changes from" value={source.value} onChange={(event) => choose(event.target.value)}>
          {sources.map((one) => (
            <option key={one.value} value={one.value}>
              {one.label}
            </option>
          ))}
        </Select>
      </div>
      <div className="job-pane__scroll">
        <UnifiedDiff files={drawn.files} />
      </div>
    </>
  );
}

export function PlanBody({ session }: { session: Session }) {
  const groups = session.work?.plan?.groups ?? [];
  const revision: PlanRevision = { groups: groups.map((group) => ({ id: group.id, agent: group.agent, title: group.title, tasks: group.tasks.map((task) => ({ id: task.id, text: task.text, state: task.state })), gone: [] })) };
  const unchanged = useUnchanged(revision);
  return (
    <div className="job-pane__scroll">
      <PlanRevisionView revision={revision} unchanged={unchanged} />
    </div>
  );
}

/** The Job's pull requests, and those of the Jobs it started with the Job each came from. */
export function PullsBody({ session }: { session: Session }) {
  const children = linkedOf(useSessions(), session);
  return (
    <ul className="sv-layer__rows job-pane__scroll" aria-label="Pull requests">
      {attachmentsOf(session, "pull_request").map((pr) => (
        <PullRow key={`${session.id}:${pr.number}`} session={session} pr={pr} />
      ))}
      {children.flatMap(({ session: child }) => attachmentsOf(child, "pull_request").map((pr) => <PullRow key={`${child.id}:${pr.number}`} session={child} pr={pr} from={child.title ?? child.id} />))}
    </ul>
  );
}

/** The issues the Job works on, claimed or cited, and those of the Jobs it started. */
export function IssuesBody({ session }: { session: Session }) {
  const children = linkedOf(useSessions(), session);
  const seen = new Set<string>();
  const issues = [session, ...children.map((one) => one.session)].flatMap((one) => attachmentsOf(one, "issue").map((issue) => ({ issue, manifestId: one.manifestId }))).filter(({ issue }) => !seen.has(issue.number) && seen.add(issue.number));
  return (
    <ul className="sv-rows job-pane__scroll" aria-label="Issues">
      {issues.map(({ issue, manifestId }) => (
        <li key={issue.number}>
          <IssueRow issue={issue} manifestId={manifestId} />
        </li>
      ))}
    </ul>
  );
}

/** The Jobs linked to this one: the ones it started, the one that started it and the one it was forked from. */
export function JobsBody({ session, onOpenSession }: { session: Session; onOpenSession: (id: string) => void }) {
  const jobs = linkedSessionsOf(session, useSessions());
  return (
    <ul className="job-cards job-pane__scroll" aria-label="Jobs">
      {jobs.map((job) => (
        <li key={job.id}>
          <IssueJobCard session={job} onOpen={onOpenSession} />
        </li>
      ))}
    </ul>
  );
}

/** Everything attached to the Job, by kind, newest first, each opening as it does in the Artifacts layer. */
export function LedgerBody({ session, entries, onOpenSession }: { session: Session; entries: readonly LedgerEntry[]; onOpenSession: (id: string) => void }) {
  const sessions = useSessions();
  const groups = useMemo(() => artifactsOf(session, entries, { sessions, open: onOpenSession }), [session, entries, sessions, onOpenSession]);
  return (
    <div className="job-pane__scroll">
      <Groups groups={groups} session={session} />
    </div>
  );
}

/** **The mock's**: a small fixed tree standing for the Job's worktree until Fleet serves one. */
type Node = { name: string; children?: readonly Node[] };
const MOCK_TREE: readonly Node[] = [
  { name: "crates", children: [{ name: "fleet", children: [{ name: "src", children: [{ name: "lease.rs" }, { name: "pool.rs" }] }] }] },
  { name: "apps", children: [{ name: "desktop", children: [{ name: "src", children: [{ name: "BayRows.tsx" }] }] }] },
  { name: "README.md" },
];

function Entry({ node, path, session, depth }: { node: Node; path: string; session: Session; depth: number }) {
  const [open, setOpen] = useState(depth < 2);
  const at = `${path}${node.name}`;
  const row = { paddingInlineStart: `calc(var(--space-3) * ${depth + 1})` };
  if (node.children === undefined) {
    return (
      <li role="treeitem" aria-selected={false}>
        <button type="button" className="job-tree__row" style={row} aria-label={`Open ${at}`} onClick={() => openInPanel({ kind: "file", path: at, from: "session", sessionId: session.id })}>
          <File size={16} strokeWidth={2} aria-hidden />
          <span>{node.name}</span>
        </button>
      </li>
    );
  }
  const Glyph = open ? FolderOpen : Folder;
  return (
    <li role="treeitem" aria-expanded={open}>
      <button type="button" className="job-tree__row" style={row} aria-label={node.name} onClick={() => setOpen(!open)}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
        <span>{node.name}</span>
      </button>
      {open ? (
        <ul role="group">
          {node.children.map((one) => (
            <Entry key={one.name} node={one} path={`${at}/`} session={session} depth={depth + 1} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/** The Job's worktree as a read-only tree; a file opens read-only. */
export function FilesBody({ session }: { session: Session }) {
  return (
    <ul role="tree" className="job-tree job-pane__scroll" aria-label="Files">
      {MOCK_TREE.map((one) => (
        <Entry key={one.name} node={one} path="" session={session} depth={0} />
      ))}
    </ul>
  );
}

/** The Requests panel itself, for this Job: its list, its sending and the answer, with no sheet around it. */
export function RequestBody({ session }: { session: Session }) {
  return (
    <InlinePanel.Provider value>
      <div className="job-pane__panel">
        <RequestsPanel item={{ kind: "requests", sessionId: session.id }} />
      </div>
    </InlinePanel.Provider>
  );
}

/**
 * The Cockpit's Terminal tile and a Terminal tab are one shell. A shell shown with no tab for it opens one, as a tab the Job opens itself does; the last tab closed puts the shell away.
 * Draws nothing.
 */
export function TerminalSync({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const shells = draft?.terminalRuns !== undefined ? shellsOf(window.armada) : null;
  const { shown } = useJobShell(shells, "page", jobId);
  const has = hasKind(useJobPanes(jobId).arrangement, "terminal");
  const was = useRef(has);
  useEffect(() => {
    const closed = was.current && !has;
    was.current = has;
    if (closed) return void shells?.show("page", jobId, false);
    if (shown && !has) jobOpens(jobId, "terminal");
  }, [shown, has]);
  return null;
}
