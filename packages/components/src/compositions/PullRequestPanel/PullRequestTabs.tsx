import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { Ban, Check, ChevronDown, ChevronRight, ChevronsDownUp, ChevronsUpDown, CornerUpRight, ExternalLink, FileText, FilePenLine, Folder, FolderOpen, LoaderCircle, MessageSquareDiff, MessageSquareOff, MessageSquareReply, ScrollText, Send, ShieldCheck, ShieldOff, ShieldX, SquarePlus, Trash2 } from "lucide-react";
import type { PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestFile, PullRequestPage } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Stamp, StampSurface } from "../../primitives/Stamp/Stamp";
import { TabPanel, Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { CommentRead } from "../LaneCanvas/model";
import { ranFor } from "./pull-request-format";
import { ReachTests, ReachToggle, type FilesReach, type ReachTest } from "./PullRequestReach";

/**
 * The four tabs under the panel's acts: the conversation, the checks, the files and the commits,
 * each drawn from the page Fleet served. **No count sits beside a tab or a list.** An absent field
 * draws nothing: a check that did not say how long it ran has no figure, a commit with no author
 * has none.
 */
export type PullRequestTabsProps = {
  page: Pick<PullRequestPage, "author" | "created_at" | "conversation" | "checks" | "files" | "commits">;
  /** What its author wrote of it, as Markdown. The first entry of the conversation. Absent or blank draws none. */
  description?: string;
  onOpenLink: (address: string) => void;
  /** The Files tab's body, where the host draws each file's diff. Absent draws the list of files. */
  files?: ReactNode;
  /** A tab of the host's own, last in the strip: a pull request's review tools, drawn by the surface that has them. */
  more?: { id: string; label: string; body: ReactNode };
  /** The tests that reach each changed file, drawn on the file list when `files` is absent. Absent draws none. */
  reach?: FilesReach | undefined;
  /** A test opened from under its file. */
  onTest?: ((test: ReachTest) => void) | undefined;
  /** The tab shown, where the host holds it. Absent leaves the tabs to choose for themselves. */
  tab?: string;
  onTab?: (id: string) => void;
  /** What Fleet's agent made of an entry, where it read it as a human comment on the pull request: its mark, and the acts on it. */
  triage?: (entry: PullRequestEntry) => EntryTriage | undefined;
  /** The entry the panel was opened at: scrolled to and marked. */
  focused?: (entry: PullRequestEntry) => boolean;
};

/** What the agent made of one human comment, and what may be done with it. */
export type EntryTriage = {
  read: CommentRead;
  /** Where in the code it was left. */
  where?: string;
  /** It was sent to the Job. */
  sent?: boolean;
  /** Hands it to the Job's agent. Absent, the comment has no act. */
  onSend?: () => void;
};

/** What the agent made of a comment, as a glyph and the words its tooltip reads. */
export const COMMENT_READ: Readonly<Record<CommentRead, { Glyph: LucideIcon; said: string }>> = {
  change: { Glyph: MessageSquareDiff, said: "Needs a change" },
  answer: { Glyph: MessageSquareReply, said: "Answer only" },
  skip: { Glyph: MessageSquareOff, said: "Not worth acting on" },
};

const TABS = [
  { id: "conversation", label: "Conversation" },
  { id: "checks", label: "Checks" },
  { id: "files", label: "Files" },
  { id: "commits", label: "Commits" },
];

export function PullRequestTabs({ page, description, onOpenLink, files, more, reach, onTest, tab: held, onTab, triage, focused }: PullRequestTabsProps) {
  const [own, setOwn] = useState("conversation");
  const tab = held ?? own;
  const setTab = onTab ?? setOwn;
  const items = more === undefined ? TABS : [...TABS, { id: more.id, label: more.label }];
  return (
    <div className="armada-pr-tabs">
      <Tabs items={items} value={tab} onChange={setTab} />
      <TabPanel tab={tab} role="tabpanel" aria-label={items.find((one) => one.id === tab)?.label} className="armada-pr-tabs__panel">
        {tab === "conversation" ? <Conversation page={page} description={description ?? ""} {...(triage === undefined ? {} : { triage })} {...(focused === undefined ? {} : { focused })} /> : null}
        {tab === "checks" ? <Checks rows={page.checks} onOpenLink={onOpenLink} /> : null}
        {tab === "files" ? (files ?? <FileRows rows={page.files} reach={reach} onTest={onTest} />) : null}
        {tab === "commits" ? <Commits rows={page.commits} /> : null}
        {more !== undefined && tab === more.id ? more.body : null}
      </TabPanel>
    </div>
  );
}

// The registry has no glyph for a file's status; these are the nearest it holds, named once here.
// REPORTED: add one per status to `packages/icons/icons`.
const VERDICT: Record<Extract<PullRequestEntry, { kind: "review" }>["verdict"], { Glyph: LucideIcon; said: string; tone?: string }> = {
  approved: { Glyph: Check, said: "Approved", tone: "success" },
  changes_requested: { Glyph: Ban, said: "Changes requested", tone: "failed" },
  commented: { Glyph: ScrollText, said: "Commented" },
};

const CHECK: Record<PullRequestCheckRow["state"], { Glyph: LucideIcon; said: string; tone: string }> = {
  passed: { Glyph: ShieldCheck, said: "Passed", tone: "success" },
  failed: { Glyph: ShieldX, said: "Failed", tone: "failed" },
  running: { Glyph: LoaderCircle, said: "Running", tone: "running" },
  skipped: { Glyph: ShieldOff, said: "Skipped", tone: "neutral" },
};

const FILE: Record<PullRequestFile["status"], { Glyph: LucideIcon; said: string }> = {
  added: { Glyph: SquarePlus, said: "Added" },
  modified: { Glyph: FilePenLine, said: "Modified" },
  removed: { Glyph: Trash2, said: "Removed" },
  renamed: { Glyph: CornerUpRight, said: "Renamed" },
};

/** The lines added and removed, in the hues a diff takes. Absent for neither draws nothing. */
export function DiffSize({ additions, deletions }: { additions: number; deletions: number }) {
  return (
    <Tooltip label="Lines added and removed">
      <span className="armada-pr-diff mono">
        <span className="armada-pr-diff__added">{`+${additions}`}</span>
        <span className="armada-pr-diff__removed">{`−${deletions}`}</span>
      </span>
    </Tooltip>
  );
}

export function Mark({ Glyph, said, tone }: { Glyph: LucideIcon; said: string; tone?: string | undefined }) {
  return (
    <Tooltip label={said}>
      <span className="armada-pr-mark" data-tone={tone} role="img" aria-label={said}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The agent's read of a human comment, on the comment: its glyph, where in the code it was left, and the act that hands it to the Job, or the mark that it was handed. */
function Triaged({ triage, who }: { triage: EntryTriage; who: string }) {
  const { Glyph, said } = COMMENT_READ[triage.read];
  return (
    <span className="armada-pr-card__triage" data-read={triage.read}>
      <Mark Glyph={Glyph} said={said} />
      {triage.where === undefined ? null : <code className="armada-pr-card__where">{triage.where}</code>}
      {triage.sent === true ? (
        <Mark Glyph={Check} said="Sent to the Job" tone="success" />
      ) : triage.onSend === undefined ? null : (
        <Tooltip label="Send to Job">
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={`Send to Job: ${who}${triage.where === undefined ? "" : ` on ${triage.where}`}`} onClick={triage.onSend}>
            <Send size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
    </span>
  );
}

function Entry({ title, who, at, mark, body, triage, focus }: { title?: string; who: string | undefined; at: string | undefined; mark?: ReactNode; body: string; triage?: EntryTriage | undefined; focus?: boolean }) {
  const card = useRef<HTMLElement>(null);
  useEffect(() => {
    if (focus === true) card.current?.scrollIntoView({ block: "nearest" });
  }, [focus]);
  return (
    <li className="armada-pr-entry">
      <article ref={card} className="armada-pr-card" aria-label={title ?? who} data-focus={focus || undefined}>
        <header className="armada-pr-card__head">
          {mark}
          {who === undefined ? null : <b className="armada-pr-card__who">{who}</b>}
          {at === undefined ? null : <Stamp first="age" at={at} className="armada-pr-card__ago" />}
          {triage === undefined ? null : <Triaged triage={triage} who={who ?? ""} />}
        </header>
        {body.trim() === "" ? null : (
          <div className="armada-pr-card__body">
            <Prose text={body} />
          </div>
        )}
      </article>
    </li>
  );
}

export function Conversation({ page, description, triage, focused }: { page: PullRequestTabsProps["page"]; description: string; triage?: PullRequestTabsProps["triage"]; focused?: PullRequestTabsProps["focused"] }) {
  const first = description.trim() !== "";
  if (!first && page.conversation.length === 0) return null;
  return (
    <StampSurface>
      <ul className="armada-pr-list" aria-label="Conversation">
        {first ? (
          <Entry
            title="Description"
            who={page.author}
            at={page.created_at}
            mark={<Mark Glyph={FileText} said="Description" />}
            body={description}
          />
        ) : null}
        {page.conversation.map((entry, at) => {
          const verdict = entry.kind === "review" ? VERDICT[entry.verdict] : undefined;
          return (
            <Entry
              key={`${at}:${entry.at}`}
              who={entry.author}
              at={entry.at}
              mark={verdict === undefined ? undefined : <Mark Glyph={verdict.Glyph} said={verdict.said} tone={verdict.tone} />}
              body={entry.body}
              triage={triage?.(entry)}
              focus={focused?.(entry)}
            />
          );
        })}
      </ul>
    </StampSurface>
  );
}

export function Checks({ rows, onOpenLink }: { rows: readonly PullRequestCheckRow[]; onOpenLink: (address: string) => void }) {
  if (rows.length === 0) return null;
  return (
    <ul className="armada-pr-list armada-pr-rows" aria-label="Checks">
      {rows.map((row, at) => {
        const state = CHECK[row.state];
        return (
          <li key={`${at}:${row.name}`} className="armada-pr-row" data-columns="check">
            <Mark Glyph={state.Glyph} said={state.said} tone={state.tone} />
            <span className="armada-pr-row__name mono">{row.name}</span>
            {row.seconds === undefined ? (
              <span />
            ) : (
              <Tooltip label="Ran for">
                <span className="armada-pr-row__figure mono">{ranFor(row.seconds)}</span>
              </Tooltip>
            )}
            {row.address === undefined ? (
              <span />
            ) : (
              <Tooltip label="Open the log">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Open the log of ${row.name}`} onClick={() => onOpenLink(row.address!)}>
                  <ExternalLink size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            )}
          </li>
        );
      })}
    </ul>
  );
}

/** A file's status as its glyph, with the word in its tooltip. */
export function FileMark({ status }: { status: PullRequestFile["status"] }) {
  const { Glyph, said } = FILE[status];
  return <Mark Glyph={Glyph} said={said} />;
}

type FileTreeRow =
  | { kind: "dir"; key: string; name: string; depth: number; above: string[] }
  | { kind: "file"; key: string; name: string; depth: number; above: string[]; row: PullRequestFile };
type FileDir = { dirs: Map<string, FileDir>; files: { name: string; row: PullRequestFile }[] };

/** The paths as a tree, flattened in reading order: folders before files, a folder holding only one folder drawn with it as `a/b/c`. `above` is every folder a row sits under. */
function fileTree(rows: readonly PullRequestFile[]): FileTreeRow[] {
  const root: FileDir = { dirs: new Map(), files: [] };
  for (const row of rows) {
    const parts = row.path.split("/");
    const name = parts.pop() ?? row.path;
    let dir = root;
    for (const part of parts) {
      let next = dir.dirs.get(part);
      if (next === undefined) dir.dirs.set(part, (next = { dirs: new Map(), files: [] }));
      dir = next;
    }
    dir.files.push({ name, row });
  }
  const out: FileTreeRow[] = [];
  const walk = (dir: FileDir, depth: number, parent: string, above: string[]) => {
    for (const [first, child] of [...dir.dirs].sort(([a], [b]) => a.localeCompare(b))) {
      let name = first;
      let node = child;
      while (node.files.length === 0 && node.dirs.size === 1) {
        const [[only, next]] = [...node.dirs] as [[string, FileDir]];
        name = `${name}/${only}`;
        node = next;
      }
      const key = `${parent}${name}/`;
      out.push({ kind: "dir", key, name, depth, above });
      walk(node, depth + 1, key, [...above, key]);
    }
    for (const file of [...dir.files].sort((a, b) => a.name.localeCompare(b.name)))
      out.push({ kind: "file", key: file.row.path, name: file.name, depth, above, row: file.row });
  };
  walk(root, 0, "", []);
  return out;
}

/**
 * The files as a tree of folders, each file its name, status and lines added and removed. A folder
 * folds away with its chevron, or with ← and → while it has focus, and the head folds or opens them all.
 * With `onFile` each file is a press, and `current` is marked.
 *
 * With `reach` a file the map was asked about carries its mark, and → opens the tests under it (← closes
 * them). ↑ and ↓ move between every row, tests included, and Enter on a test that is a file opens it.
 */
export function FileRows({
  rows,
  current,
  onFile,
  reach,
  onTest,
}: {
  rows: readonly PullRequestFile[];
  current?: string | undefined;
  onFile?: (path: string) => void;
  reach?: FilesReach | undefined;
  onTest?: ((test: ReachTest) => void) | undefined;
}) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  if (rows.length === 0) return null;
  const tree = fileTree(rows);
  const folders = tree.filter((node) => node.kind === "dir").map((node) => node.key);
  const toggle = (key: string, fold: boolean) =>
    setFolded((was) => {
      const next = new Set(was);
      if (fold) next.add(key);
      else next.delete(key);
      return next;
    });
  const allFolded = folders.length > 0 && folders.every((key) => folded.has(key));
  const setOpen = (path: string, open: boolean) =>
    setOpened((was) => {
      const next = new Set(was);
      if (open) next.add(path);
      else next.delete(path);
      return next;
    });
  // Every row a key can land on, in the order drawn. The target is read off the event: a key sent to a row moves from that row.
  const move = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const here = (event.target as HTMLElement).closest<HTMLElement>("[data-nav]");
    if (here === null) return;
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>("[data-nav]")];
    const next = all[all.indexOf(here) + (event.key === "ArrowDown" ? 1 : -1)];
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  };
  return (
    <div className="armada-pr-tree">
      {folders.length === 0 ? null : (
        <div className="armada-pr-tree__head">
          <span className="armada-pr-tree__count">{`${rows.length} ${rows.length === 1 ? "file" : "files"}`}</span>
          <Tooltip label={allFolded ? "Open every folder" : "Fold every folder"}>
            <button
              type="button"
              className="armada-pr-tree__fold"
              aria-label={allFolded ? "Open every folder" : "Fold every folder"}
              onClick={() => setFolded(allFolded ? new Set() : new Set(folders))}
            >
              {allFolded ? <ChevronsUpDown size={16} strokeWidth={2} aria-hidden /> : <ChevronsDownUp size={16} strokeWidth={2} aria-hidden />}
            </button>
          </Tooltip>
        </div>
      )}
      <ul className="armada-pr-list armada-pr-tree__list" aria-label="Files" onKeyDown={move}>
        {tree
          .filter((node) => !node.above.some((key) => folded.has(key)))
          .map((node) => {
            const indent = { "--depth": node.depth } as CSSProperties;
            if (node.kind === "dir") {
              const open = !folded.has(node.key);
              const Chevron = open ? ChevronDown : ChevronRight;
              const Shape = open ? FolderOpen : Folder;
              return (
                <li key={node.key} className="armada-pr-tree__row" style={indent}>
                  <button
                    type="button"
                    className="armada-pr-tree__item"
                    data-kind="dir"
                    data-nav
                    aria-expanded={open}
                    onClick={() => toggle(node.key, open)}
                    onKeyDown={(event) => {
                      if (event.key === "ArrowLeft" && open) toggle(node.key, true);
                      else if (event.key === "ArrowRight" && !open) toggle(node.key, false);
                      else return;
                      event.preventDefault();
                    }}
                  >
                    <Chevron className="armada-pr-tree__lead" size={16} strokeWidth={2} aria-hidden />
                    <Shape className="armada-pr-tree__folder" size={16} strokeWidth={2} aria-hidden />
                    <span className="armada-pr-tree__name mono" title={`${node.name}/`}>{`${node.name}/`}</span>
                  </button>
                </li>
              );
            }
            const { row } = node;
            const reached = reach?.[row.path];
            const isOpen = reached !== undefined && opened.has(row.path);
            // ← and → on a file, as on a folder, close and open what is under it.
            const keys = (event: KeyboardEvent<HTMLElement>) => {
              if (reached === undefined) return;
              if (event.key === "ArrowRight" && !isOpen) setOpen(row.path, true);
              else if (event.key === "ArrowLeft" && isOpen) setOpen(row.path, false);
              else return;
              event.preventDefault();
            };
            const cells = (
              <>
                <span className="armada-pr-tree__lead">
                  <FileMark status={row.status} />
                </span>
                <span className="armada-pr-tree__name mono" title={row.path}>
                  {node.name}
                </span>
                <DiffSize additions={row.additions} deletions={row.deletions} />
              </>
            );
            // The whole row is the control.
            return (
              <li key={node.key} className="armada-pr-tree__row" style={indent} data-reach={reached !== undefined || undefined}>
                {onFile === undefined ? (
                  <div className="armada-pr-tree__item" data-kind="file" data-nested={node.depth > 0 || undefined} data-nav tabIndex={-1} onKeyDown={keys}>
                    {cells}
                  </div>
                ) : (
                  <button
                    type="button"
                    className="armada-pr-tree__item"
                    data-kind="file"
                    data-nav
                    data-nested={node.depth > 0 || undefined}
                    data-on={row.path === current || undefined}
                    aria-current={row.path === current || undefined}
                    onClick={() => onFile(row.path)}
                    onKeyDown={keys}
                  >
                    {cells}
                  </button>
                )}
                {reached === undefined ? null : <ReachToggle name={node.name} reach={reached} open={isOpen} onToggle={() => setOpen(row.path, !isOpen)} />}
                {reached === undefined || !isOpen ? null : <ReachTests name={node.name} reach={reached} nested={node.depth > 0} onTest={onTest} onClose={() => setOpen(row.path, false)} />}
              </li>
            );
          })}
      </ul>
    </div>
  );
}

export function Commits({ rows }: { rows: readonly PullRequestCommit[] }) {
  if (rows.length === 0) return null;
  return (
    <ul className="armada-pr-list armada-pr-rows" aria-label="Commits">
      {rows.map((row) => (
        <li key={row.sha} className="armada-pr-row" data-columns="commit">
          <Tooltip label="Commit">
            <span className="armada-pr-row__sha mono">{row.sha}</span>
          </Tooltip>
          <span className="armada-pr-row__name">{row.headline}</span>
          {row.author === undefined ? <span /> : <span className="armada-pr-row__author">{row.author}</span>}
        </li>
      ))}
    </ul>
  );
}
