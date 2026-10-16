// A pull request as what it makes happen: themes, each a one-line purpose over its changes in the order they were
// made, and each change with why it was made. **The hunk is the evidence and stays folded** until he opens it;
// a path is only a small caption. The same block draws a change three ways, for the explanation, for what a tool
// found on it, and for the Comments tool, so a finding, a thread and his own words all hang on the change they are about.

import { useState } from "react";
import { ChevronDown, ChevronRight, MessageSquare } from "lucide-react";
import { Button, DiffSize, Prose, PrMark, Tooltip, UnifiedDiff } from "@armada/components";
import type { DiffFile } from "@armada/components";

import type { useSendFindings } from "./acts";
import { DraftCard, FindingCard, LineComposer, ThreadCard } from "./lines";
import { TOOL_MARK } from "./marks";
import type { Anchor, Change, Draft, Finding, ReviewPr, Theme, Thread } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { toolsOf } from "./tools";

export type ChangeMode = "explain" | "result" | "comments";

type Send = ReturnType<typeof useSendFindings>;

const hereOf = (at: Anchor, hunk: number, line: number | undefined) => at.hunk === hunk && at.line === line;

/** One theme: its title and purpose over the changes, in the order they were made. */
export function ThemeBlock({ pr, theme, number, mode, tool, send, quiet }: { pr: ReviewPr; theme: Theme; number: number; mode: ChangeMode; tool?: string; send: Send; quiet?: (change: Change) => boolean }) {
  const changes = quiet === undefined ? theme.changes : theme.changes.filter((one) => !quiet(one));
  if (changes.length === 0) return null;
  return (
    <section className="pr-theme" aria-label={`Theme: ${theme.title}`}>
      <header className="pr-theme__head">
        <span className="pr-theme__n mono">{number}</span>
        <h3 className="pr-theme__title">{theme.title}</h3>
      </header>
      <p className="pr-theme__purpose">{theme.purpose}</p>
      <ol className="pr-theme__changes">
        {changes.map((one) => (
          <li key={one.id}>
            <ChangeBlock pr={pr} change={one} number={theme.changes.indexOf(one) + 1} mode={mode} {...(tool === undefined ? {} : { tool })} send={send} />
          </li>
        ))}
      </ol>
    </section>
  );
}

/**
 * The changes of a pull request nobody has explained: no themes and no why, only where each one is, in file order. A tool
 * pressed here pins its findings to the path and the lines, with the hunk one press away.
 */
export function BareChanges({ pr, mode, tool, send, quiet }: { pr: ReviewPr; mode: ChangeMode; tool?: string; send: Send; quiet?: (change: Change) => boolean }) {
  const all = pr.explanation.themes.flatMap((theme) => theme.changes);
  const shown = [...all].sort((a, b) => (a.hunks[0]?.path ?? "").localeCompare(b.hunks[0]?.path ?? "")).filter((one) => quiet === undefined || !quiet(one));
  if (shown.length === 0) return null;
  return (
    <ol className="pr-bare">
      {shown.map((one, at) => (
        <li key={one.id}>
          <ChangeBlock pr={pr} change={one} number={at + 1} mode={mode} bare {...(tool === undefined ? {} : { tool })} send={send} />
        </li>
      ))}
    </ol>
  );
}

function ChangeBlock({ pr, change, number, mode, tool, send, bare = false }: { pr: ReviewPr; change: Change; number: number; mode: ChangeMode; tool?: string; send: Send; bare?: boolean }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const tools = toolsOf(reviews, pr.manifestId);
  const [composing, setComposing] = useState<(Anchor & { thread?: string }) | undefined>(undefined);

  const threads = work.threads.filter((one) => one.change === change.id);
  const drafts = work.drafts.filter((one) => one.change === change.id);
  const findings = work.findings.filter((one) => one.change === change.id && !one.dismissed);
  const shownFindings = tool === undefined ? [] : findings.filter((one) => one.tool === tool);
  const onLines = threads.some((one) => one.line !== undefined) || drafts.some((one) => one.line !== undefined && one.thread === undefined);
  // Comments opens the hunks that have words on their lines, so a thread sits on the line it is about.
  const auto = mode === "comments" && onLines;
  const shown = auto ? !work.opened.includes(change.id) : work.opened.includes(change.id);
  const marked = [...new Set(findings.map((one) => tools.find((def) => def.id === one.tool)?.mark ?? "custom"))];
  const open = threads.some((one) => !one.resolved) || drafts.length > 0;
  const place = change.hunks[0]?.path ?? change.title;

  const cards = (here: (one: Anchor) => boolean) => {
    const ts = threads.filter(here);
    const ds = drafts.filter((one) => here(one) && one.thread === undefined);
    const writing = composing !== undefined && here(composing) && composing.thread === undefined;
    if (ts.length + ds.length === 0 && !writing) return null;
    return (
      <div className="pr-under">
        {ts.map((thread) => (
          <ThreadBlock key={thread.id} pr={pr} change={change} thread={thread} composing={composing} setComposing={setComposing} />
        ))}
        {ds.map((draft) => (
          <DraftCard key={draft.id} draft={draft} onRemove={() => store.removeDraft(pr.key, draft.id)} />
        ))}
        {writing ? (
          <LineComposer
            keep={`pr-review:${pr.key}:${change.id}:${composing.hunk}:${composing.line ?? "change"}`}
            label="Comment"
            onAdd={(body) => {
              store.addDraft(pr.key, { change: change.id, hunk: composing.hunk, ...(composing.line === undefined ? {} : { line: composing.line }) }, body);
              setComposing(undefined);
            }}
            onCancel={() => setComposing(undefined)}
          />
        ) : null}
      </div>
    );
  };

  return (
    <article className="pr-change" aria-label={`Change: ${bare ? place : change.title}`} data-mode={mode} data-bare={bare || undefined}>
      <header className="pr-change__head">
        {bare ? null : <span className="pr-change__n mono">{number}</span>}
        <h4 className="pr-change__title" data-mono={bare || undefined}>
          {bare ? place : change.title}
        </h4>
        <span className="pr-change__marks">
          {marked.map((one) => (
            <PrMark key={one} Glyph={TOOL_MARK[one].Glyph} said={TOOL_MARK[one].said} />
          ))}
          {open ? <PrMark Glyph={MessageSquare} said="Open comments" /> : null}
        </span>
        {mode === "comments" ? (
          <Tooltip label="Comment on this change">
            <Button variant="ghost" size="sm" aria-label={`Comment on ${bare ? place : change.title}`} onClick={() => setComposing({ change: change.id, hunk: 0 })}>
              Comment
            </Button>
          </Tooltip>
        ) : null}
      </header>
      <p className="pr-change__path mono">{bare ? (change.hunks[0]?.lines[0]?.text ?? "") : change.hunks.map((one) => one.path).filter((one, at, all) => all.indexOf(one) === at).join("  ")}</p>
      {mode === "explain" && !bare ? (
        <div className="pr-change__why">
          <Prose text={change.why} />
        </div>
      ) : null}
      {mode === "result"
        ? shownFindings.map((finding) => (
            <FindingBlock key={finding.id} pr={pr} finding={finding} mark={tools.find((def) => def.id === finding.tool)?.mark ?? "custom"} send={send} />
          ))
        : null}
      {mode === "comments" ? cards((one) => one.line === undefined) : null}
      <Button variant="ghost" size="sm" className="pr-change__toggle" aria-expanded={shown} onClick={() => store.toggleOpen(pr.key, change.id)}>
        {shown ? <ChevronDown size={16} strokeWidth={2} aria-hidden /> : <ChevronRight size={16} strokeWidth={2} aria-hidden />}
        {shown ? "Hide change" : "Show change"}
      </Button>
      {shown
        ? change.hunks.map((hunk, at) => {
            const file: DiffFile = { path: hunk.path, lines: hunk.lines, meta: <DiffSize additions={hunk.lines.filter((one) => one.kind === "added").length} deletions={hunk.lines.filter((one) => one.kind === "removed").length} /> };
            return (
              <div key={at} className="pr-change__hunk">
                <UnifiedDiff
                  files={[file]}
                  under={(_, index) => cards((one) => hereOf(one, at, index))}
                  onLine={{
                    label: "Comment on this line",
                    name: (_, index) => `Comment on ${hunk.lines[index]?.text.trim() ?? "this line"}`,
                    press: (_, index) => setComposing({ change: change.id, hunk: at, line: index }),
                  }}
                />
              </div>
            );
          })
        : null}
    </article>
  );
}

function ThreadBlock({ pr, change, thread, composing, setComposing }: { pr: ReviewPr; change: Change; thread: Thread; composing: (Anchor & { thread?: string }) | undefined; setComposing: (next: (Anchor & { thread?: string }) | undefined) => void }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const drafts: Draft[] = work.drafts.filter((one) => one.thread === thread.id);
  return (
    <ThreadCard
      thread={thread}
      where={change.title}
      drafts={drafts}
      replying={composing?.thread === thread.id}
      onReply={() => setComposing({ change: thread.change, hunk: thread.hunk, ...(thread.line === undefined ? {} : { line: thread.line }), thread: thread.id })}
      onResolve={(resolved) => store.resolve(pr.key, thread.id, resolved)}
      onRemove={(id) => store.removeDraft(pr.key, id)}
      composer={
        <LineComposer
          keep={`pr-review:${pr.key}:reply:${thread.id}`}
          label="Reply"
          onAdd={(body) => {
            store.addDraft(pr.key, thread, body, thread.id);
            setComposing(undefined);
          }}
          onCancel={() => setComposing(undefined)}
        />
      }
    />
  );
}

function FindingBlock({ pr, finding, mark, send }: { pr: ReviewPr; finding: Finding; mark: Parameters<typeof FindingCard>[0]["mark"]; send: Send }) {
  const store = useReviewStore()!;
  return <FindingCard finding={finding} mark={mark} session={send !== undefined} onDismiss={() => store.dismiss(pr.key, finding.id)} onSend={() => send?.([finding])} />;
}
