// A pull request as what it makes happen: themes, each a one-line purpose over its changes in the order they were
// made, and each change with why it was made. **The hunk is the evidence and stays folded** until he opens it;
// a path is only a small caption. The same block draws a change three ways, for the explanation, for what a tool
// found on it, and for the Comments tool, so a finding, a thread and his own words all hang on the change they are about.

import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight, LoaderCircle, MessageSquare } from "lucide-react";
import { Button, DiffSize, Prose, PrMark, Tooltip, UnifiedDiff } from "@armada/components";
import type { DiffFile } from "@armada/components";

import type { useSendFindings } from "./acts";
import { changesOf, fileOf, ownerOf, placeOfDraft, placeOfFinding, placeOfThread, spotOf } from "./changes";
import type { Spot } from "./changes";
import { DraftCard, FindingCard, LineComposer, ThreadCard } from "./lines";
import { TOOL_MARK } from "./marks";
import type { Change, Draft, Finding, ReviewPr, Theme, Thread } from "./model";
import { placeOfLine } from "./patch";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { toolsOf } from "./tools";

export type ChangeMode = "explain" | "result" | "comments";

type Send = ReturnType<typeof useSendFindings>;

/** Where he is writing: a hunk of the change and a line of it, or the change itself; or a reply on a thread. */
type Composing = Spot & { thread?: string };

const hereOf = (at: Spot | undefined, hunk: number, line: number | undefined) => at !== undefined && at.hunk === hunk && at.line === line;

/** One theme: its title and purpose over the changes, in the order they were made. */
export function ThemeBlock({ pr, theme, number, mode, tool, send, quiet }: { pr: ReviewPr; theme: Theme; number: number; mode: ChangeMode; tool?: string; send: Send; quiet?: (change: Change) => boolean }) {
  const changes = quiet === undefined ? theme.changes : theme.changes.filter((one) => !quiet(one));
  if (changes.length === 0) return null;
  return (
    <section className="pr-theme" aria-label={`Theme: ${theme.title}`} data-hue={(number - 1) % 5}>
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
  const all = changesOf(pr, useReviews().work[pr.key] ?? NO_WORK);
  const shown = [...all].sort((a, b) => fileOf(a).localeCompare(fileOf(b))).filter((one) => quiet === undefined || !quiet(one));
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
  const [composing, setComposing] = useState<Composing | undefined>(undefined);

  // **A thread or a draft is named by its place in a file**, so it belongs to the first change that holds that line.
  const all = changesOf(pr, work);
  const threads = work.threads.filter((one) => ownerOf(all, one.path, placeOfThread(one))?.id === change.id);
  const drafts = work.drafts.filter((one) => (one.thread === undefined ? ownerOf(all, one.path, placeOfDraft(one))?.id === change.id : threads.some((thread) => thread.id === one.thread)));
  const findings = work.findings.filter((one) => ownerOf(all, one.at.path, placeOfFinding(one))?.id === change.id && !one.dismissed);
  const shownFindings = tool === undefined ? [] : findings.filter((one) => one.tool === tool);
  const onLines = threads.some((one) => one.line !== undefined) || drafts.some((one) => one.line !== undefined && one.thread === undefined);
  // Comments opens the hunks that have words on their lines, so a thread sits on the line it is about.
  const auto = mode === "comments" && onLines;
  const shown = auto ? !work.opened.includes(change.id) : work.opened.includes(change.id);
  const marked = [...new Set(findings.map((one) => tools.find((def) => def.id === one.tool)?.mark ?? "custom"))];
  const open = threads.some((one) => !one.resolved) || drafts.length > 0;
  const place = change.hunks[0]?.path ?? change.title;
  // **A change's hunk is cut from its file's patch**, which is read from the forge the first time the change is shown.
  const path = fileOf(change);
  const patch = work.patches[path];
  useEffect(() => {
    if (shown) store.readFile(pr.key, path);
  }, [shown, store, pr.key, path]);

  const spotOfThread = (thread: Thread) => spotOf(change, thread.path, placeOfThread(thread));
  const spotOfDraft = (draft: Draft) => spotOf(change, draft.path, placeOfDraft(draft));
  const pathOf = (hunk: number) => change.hunks[hunk]?.path ?? change.hunks[0]?.path ?? change.id;

  const cards = (here: (one: Spot | undefined) => boolean) => {
    const ts = threads.filter((one) => here(spotOfThread(one)));
    const ds = drafts.filter((one) => one.thread === undefined && here(spotOfDraft(one)));
    const writing = composing !== undefined && composing.thread === undefined && here(composing);
    if (ts.length + ds.length === 0 && !writing) return null;
    return (
      <div className="pr-under">
        {ts.map((thread) => (
          <ThreadBlock key={thread.id} pr={pr} change={change} thread={thread} spot={spotOfThread(thread)} composing={composing} setComposing={setComposing} />
        ))}
        {ds.map((draft) => (
          <DraftCard key={draft.id} draft={draft} onRemove={() => store.removeDraft(pr.key, draft.id)} />
        ))}
        {writing ? (
          <LineComposer
            keep={`pr-review:${pr.key}:${change.id}:${composing.hunk}:${composing.line ?? "change"}`}
            label="Comment"
            onAdd={(body) => {
              const lines = change.hunks[composing.hunk]?.lines ?? [];
              const at = composing.line === undefined ? undefined : placeOfLine(lines, composing.line);
              store.addDraft(pr.key, { path: pathOf(composing.hunk), ...(at === undefined ? {} : { line: at.line, side: at.side }), body });
              setComposing(undefined);
            }}
            onCancel={() => setComposing(undefined)}
          />
        ) : null}
      </div>
    );
  };
  const onChange = (one: Spot | undefined) => one === undefined || one.line === undefined;

  return (
    <article className="pr-change" tabIndex={0} data-stop aria-label={`Change: ${bare ? place : change.title}`} data-mode={mode} data-bare={bare || undefined}>
      <header className="pr-change__head">
        {bare ? null : <span className="pr-change__n mono">{number}</span>}
        <h4 className="pr-change__title" data-mono={bare || undefined}>
          {bare ? place : change.title}
        </h4>
        <span className="pr-change__marks">
          {marked.map((one) => (
            <span key={one} className="pr-hued" data-tool={one}>
              <PrMark Glyph={TOOL_MARK[one].Glyph} said={TOOL_MARK[one].said} />
            </span>
          ))}
          {open ? <PrMark Glyph={MessageSquare} said="Open comments" /> : null}
        </span>
        {mode === "comments" ? (
          <Tooltip label="Comment on this change">
            <Button variant="ghost" size="sm" aria-label={`Comment on ${bare ? place : change.title}`} onClick={() => setComposing({ hunk: 0 })}>
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
      {mode === "comments" ? cards(onChange) : null}
      <Button variant="ghost" size="sm" className="pr-change__toggle" aria-expanded={shown} onClick={() => store.toggleOpen(pr.key, change.id)}>
        {shown ? <ChevronDown size={16} strokeWidth={2} aria-hidden /> : <ChevronRight size={16} strokeWidth={2} aria-hidden />}
        {shown ? "Hide diff" : "Show diff"}
      </Button>
      {shown && patch?.state === "reading" ? (
        <span className="pr-review__mark" data-checks="running">
          <PrMark Glyph={LoaderCircle} said="Reading the change" tone="running" />
        </span>
      ) : null}
      {shown && patch?.state === "said" ? <p className="pr-change__said">{patch.said}</p> : null}
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
                    press: (_, index) => setComposing({ hunk: at, line: index }),
                  }}
                />
              </div>
            );
          })
        : null}
    </article>
  );
}

function ThreadBlock({ pr, change, thread, spot, composing, setComposing }: { pr: ReviewPr; change: Change; thread: Thread; spot: Spot | undefined; composing: Composing | undefined; setComposing: (next: Composing | undefined) => void }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const drafts: Draft[] = work.drafts.filter((one) => one.thread === thread.id);
  return (
    <ThreadCard
      thread={thread}
      where={change.title}
      drafts={drafts}
      replying={composing?.thread === thread.id}
      onReply={() => setComposing({ hunk: spot?.hunk ?? 0, ...(spot?.line === undefined ? {} : { line: spot.line }), thread: thread.id })}
      onResolve={(resolved) => void store.resolve(pr.key, thread.id, resolved)}
      onRemove={(id) => store.removeDraft(pr.key, id)}
      composer={
        <LineComposer
          keep={`pr-review:${pr.key}:reply:${thread.id}`}
          label="Reply"
          onAdd={(body) => {
            store.addDraft(pr.key, { path: thread.path, body, thread: thread.id });
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
