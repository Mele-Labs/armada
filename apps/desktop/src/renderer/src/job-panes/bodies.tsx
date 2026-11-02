// The bodies of the tabs that stand on their own. A tab that needs the Job's thread or its story is drawn by `SessionWorkspace`, which holds both.

import { useEffect, useMemo, useRef, useState } from "react";
import { File, Folder, FolderOpen } from "lucide-react";
import { Button, SearchField, Select, UnifiedDiff, PlanRevisionView, useKept, useUnchanged, type LedgerEntry, type PlanRevision, ShellPanelPane } from "@armada/components";
import { attachmentsOf, linkedOf, type Session } from "@armada/screens/src/draft/sessions";

import { searchFiles } from "../commands";
import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { browserTabIds, reportTabs, useJobBrowser } from "../job-browser/store";
import { shellsOf, useJobShell } from "../job-shell";
import { useDeckShell, useShellArrivals } from "../job-shell-dock";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { fileDiffOf } from "../shared-panel/pull-request-patches";
import { usePanelReads, useReading } from "../shared-panel/reads";
import { RequestsPanel } from "../shared-panel/requests";
import { InlinePanel } from "../shared-panel/side";
import { openInPanel, type PanelItem } from "../shared-panel/store";
import { IssueRow } from "../session-view/IssuesBlock";
import { PullRow } from "../session-view/LedgerRow";
import { JudgeLooseNotes } from "../judge/notes";
import { Groups } from "../session-view/SessionLayers";
import { artifactsOf, linkedSessionsOf } from "../session-view/artifacts";
import { IssueJobCard } from "../issues-tab/marks";
import { jobOpens, useJobPanes } from "./store";
import { hasKind, paneById, paneOfTab, type Tab } from "./tree";
import { usePaneZoomed } from "./zoom";

/**
 * The tab's own page, which main lays over this body. Another tab in front puts it away and leaves it as it was, and so does another pane zoomed
 * over this one, since a page main lays over a pane would show through the pane that covers it.
 */
export function BrowserBody({ jobId, tab }: { jobId: string; tab: Tab }) {
  const browser = useJobBrowser(jobId, tab.id);
  const { arrangement } = useJobPanes(jobId);
  const zoomed = usePaneZoomed(jobId);
  const covered = zoomed !== undefined && paneById(arrangement, zoomed) !== undefined && paneOfTab(arrangement, tab.id)?.id !== zoomed;
  return browser === undefined || covered ? null : <JobBrowserPane sessionId={jobId} tabId={tab.id} browser={browser} placed="pane" />;
}

/**
 * Tells main which Browser tabs the Job has, each time they change, so each has a page and a tab closed from behind another is not left holding one.
 * Draws nothing.
 */
export function BrowserTabsSync({ jobId }: { jobId: string }) {
  const ids = browserTabIds(useJobPanes(jobId).arrangement);
  const said = ids.join(" ");
  useEffect(() => reportTabs(jobId, ids), [jobId, said]);
  return null;
}

/** The Job's shell. The Cockpit's Terminal tile shows the same shell, which is why it asks for it shown while a Terminal tab is open. */
export function TerminalBody({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const served = draft?.terminalRuns !== undefined;
  const pane = useDeckShell(jobId, served);
  const shells = served ? shellsOf(window.armada) : null;
  const { shown, windowed } = useJobShell(shells, "page", jobId);
  // As the tab opens, and again when the shell's window closes and puts it away: a tab with no screen would be blank.
  useEffect(() => {
    if (shells !== null && !shown && !windowed) shells.want("page", jobId);
  }, [shown, windowed]);
  // The shell is in its window, which is the only place it draws.
  if (windowed && shells !== null) {
    return (
      <div className="job-pane__scroll">
        <Button variant="ghost" size="sm" onClick={() => shells.moveBack(jobId, "docked")}>
          Move to the pane
        </Button>
      </div>
    );
  }
  // The tab names the shell and the pane places it, so the head keeps only what the shell itself can do.
  return pane === null ? null : (
    <ShellPanelPane phase={pane.phase} onEnd={pane.onEnd} onOpen={pane.onOpen}>
      {pane.children}
    </ShellPanelPane>
  );
}

/** Where changes can be read from: each pull request the Job opened, and each pull request of a Job it started. */
type Source = { value: string; label: string; pull: Extract<PanelItem, { kind: "pr" }> };

const pullOf = (job: Session, pr: { number: number; title: string; address: string }): Source["pull"] => ({ kind: "pr", number: pr.number, address: pr.address, title: pr.title, ...(job.manifestId === undefined ? {} : { manifestId: job.manifestId }) });

function sourcesOf(session: Session, all: readonly Session[]): Source[] {
  return [
    ...attachmentsOf(session, "pull_request").map((one): Source => ({ value: `pr:${one.number}`, label: `#${one.number} ${one.title}`, pull: pullOf(session, one) })),
    ...linkedOf(all, session).flatMap(({ session: sub }) => attachmentsOf(sub, "pull_request").map((one): Source => ({ value: `job:${sub.id}:${one.number}`, label: `${sub.title ?? sub.id} #${one.number}`, pull: pullOf(sub, one) }))),
  ];
}

/** The Job's changes, from the pull request the picker names: every file's patch as the forge has it. */
export function ChangesBody({ session }: { session: Session }) {
  const sources = sourcesOf(session, useSessions());
  const [chosen, choose] = useKept(`changes:${session.id}`, "");
  const source = sources.find((one) => one.value === chosen) ?? sources[0];
  const read = usePanelReads()?.pullRequestPatches;
  const key = source === undefined ? "none" : `${source.pull.manifestId ?? ""}#${source.pull.number}`;
  const patches = useReading(async () => (source === undefined || read === undefined ? undefined : read(source.pull)), key, 0, true);
  if (source === undefined) return null;
  const answer = patches.out ? undefined : patches.value;
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
        {answer === undefined ? null : answer.ok ? answer.files.map((file) => <UnifiedDiff key={file.path} {...fileDiffOf(file)} />) : <p className="job-pane__said">{answer.said}</p>}
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
      <JudgeLooseNotes job={session.id} />
      <Groups groups={groups} session={session} />
    </div>
  );
}

type Node = { name: string; children?: Node[] };

/** The paths as a tree, folders before files, each in name order. */
function treeOf(paths: readonly string[]): Node[] {
  const root: Node = { name: "", children: [] };
  for (const path of paths) {
    let at = root;
    const parts = path.split("/");
    parts.forEach((name, i) => {
      const last = i === parts.length - 1;
      let next = at.children!.find((one) => one.name === name && (one.children === undefined) === last);
      if (next === undefined) {
        next = last ? { name } : { name, children: [] };
        at.children!.push(next);
      }
      at = next;
    });
  }
  const order = (nodes: Node[]) => {
    nodes.sort((x, y) => Number(y.children !== undefined) - Number(x.children !== undefined) || x.name.localeCompare(y.name));
    nodes.forEach((one) => one.children && order(one.children));
  };
  order(root.children!);
  return root.children!;
}

function Entry({ node, path, session, depth, all }: { node: Node; path: string; session: Session; depth: number; all: boolean }) {
  const [open, setOpen] = useState(all || depth < 2);
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
            <Entry key={one.name} node={one} path={`${at}/`} session={session} depth={depth + 1} all={all} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/** The repository's files as a read-only tree, narrowed by what is typed; Fleet answers the first fifty paths that match. A file opens read-only. */
export function FilesBody({ session }: { session: Session }) {
  const [query, setQuery] = useKept(`files:${session.id}`, "");
  const found = useReading(() => searchFiles(query, session.manifestId), `${session.manifestId ?? ""}:${query}`, 0, true);
  const paths = found.out ? [] : (found.value ?? []);
  return (
    <>
      <div className="job-pane__pick">
        <SearchField label="Search files" value={query} onValue={setQuery} />
      </div>
      <ul role="tree" className="job-tree job-pane__scroll" aria-label="Files" key={query}>
        {treeOf(paths).map((one) => (
          <Entry key={one.name} node={one} path="" session={session} depth={0} all={query !== ""} />
        ))}
      </ul>
    </>
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
 * Hears the shells the agent opens. Draws nothing.
 */
export function TerminalSync({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const shells = draft?.terminalRuns !== undefined ? shellsOf(window.armada) : null;
  const { shown } = useJobShell(shells, "page", jobId);
  // A shell the agent opens shows in this Job, or in the window it already has, with or without a tab for it.
  useShellArrivals(shells, "page", jobId);
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
