import { useState, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { Ban, Check, CornerUpRight, ExternalLink, FileText, FilePenLine, LoaderCircle, ScrollText, ShieldCheck, ShieldOff, ShieldX, SquarePlus, Trash2 } from "lucide-react";
import type { PullRequestCheckRow, PullRequestCommit, PullRequestEntry, PullRequestFile, PullRequestPage } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { TabPanel, Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { agoOf, ranFor } from "./pull-request-format";

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
  /** The tab shown, where the host holds it. Absent leaves the tabs to choose for themselves. */
  tab?: string;
  onTab?: (id: string) => void;
};

const TABS = [
  { id: "conversation", label: "Conversation" },
  { id: "checks", label: "Checks" },
  { id: "files", label: "Files" },
  { id: "commits", label: "Commits" },
];

export function PullRequestTabs({ page, description, onOpenLink, files, more, tab: held, onTab }: PullRequestTabsProps) {
  const [own, setOwn] = useState("conversation");
  const tab = held ?? own;
  const setTab = onTab ?? setOwn;
  const items = more === undefined ? TABS : [...TABS, { id: more.id, label: more.label }];
  return (
    <div className="armada-pr-tabs">
      <Tabs items={items} value={tab} onChange={setTab} />
      <TabPanel tab={tab} role="tabpanel" aria-label={items.find((one) => one.id === tab)?.label} className="armada-pr-tabs__panel">
        {tab === "conversation" ? <Conversation page={page} description={description ?? ""} /> : null}
        {tab === "checks" ? <Checks rows={page.checks} onOpenLink={onOpenLink} /> : null}
        {tab === "files" ? (files ?? <FileRows rows={page.files} />) : null}
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

function Entry({ title, who, at, mark, body }: { title?: string; who: string | undefined; at: string | undefined; mark?: ReactNode; body: string }) {
  const ago = agoOf(at);
  return (
    <li className="armada-pr-entry">
      <article className="armada-pr-card" aria-label={title ?? who}>
        <header className="armada-pr-card__head">
          {mark}
          {who === undefined ? null : <b className="armada-pr-card__who">{who}</b>}
          {ago === undefined ? null : <span className="armada-pr-card__ago">{ago}</span>}
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

export function Conversation({ page, description }: { page: PullRequestTabsProps["page"]; description: string }) {
  const first = description.trim() !== "";
  if (!first && page.conversation.length === 0) return null;
  return (
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
          />
        );
      })}
    </ul>
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

/** The files, each its path, status and lines added and removed. With `onFile` each row is a press, and `current` is marked. */
export function FileRows({ rows, current, onFile }: { rows: readonly PullRequestFile[]; current?: string | undefined; onFile?: (path: string) => void }) {
  if (rows.length === 0) return null;
  return (
    <ul className="armada-pr-list armada-pr-rows" aria-label="Files">
      {rows.map((row) => {
        const cells = (
          <>
            <FileMark status={row.status} />
            <span className="armada-pr-row__name mono">{row.path}</span>
            <DiffSize additions={row.additions} deletions={row.deletions} />
          </>
        );
        // The whole row is the control.
        return onFile === undefined ? (
          <li key={row.path} className="armada-pr-row" data-columns="file">
            {cells}
          </li>
        ) : (
          <li key={row.path} className="armada-pr-row" data-press>
            <button
              type="button"
              className="armada-pr-row__button"
              data-columns="file"
              data-on={row.path === current || undefined}
              aria-current={row.path === current || undefined}
              onClick={() => onFile(row.path)}
            >
              {cells}
            </button>
          </li>
        );
      })}
    </ul>
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
