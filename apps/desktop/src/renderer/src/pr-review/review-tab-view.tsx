// The Review tab: one changed file at a time, with each finding pinned in a margin beside the lines it is about. **The finding in
// focus is tied to its lines** by a leader and a wash in its tool's hue; one he has marked Expected, Not valid or Commented dims, and
// its tie goes grey. The forge's threads hang under their lines, a resolved one folded, and one on the whole file sits over the diff.
// **Previous and Next step through the files that need him**, those with findings or open threads; Go to file reaches the rest,
// which say they are not counted. Opening it runs the repository's default tools that have not run on this commit, asking first
// (`opening-run.tsx`), and its head runs any tool or explanation again. A tool's findings land here with no tool tab open; an
// explanation opens its tab when done. A press in the gutter picks a line, Shift and a press picks the run to it, and the box under
// the run starts a review or adds to it. The bar at the foot holds his pending review and Submit.

import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import {
  Button,
  Input,
  Kbd,
  Tooltip,
  UnifiedDiff,
  scrollWithin,
  type DiffFile,
} from "@armada/components";

import type { useSendFindings } from "./acts";
import { placeOfDraft } from "./changes";
import { CodeRefs } from "./code-refs";
import { YourReview } from "./comments";
import { ThreadHere } from "./comments-pane";
import {
  OpeningAsk,
  asksOf,
  defaultsOf,
  headTools,
  runOf,
  runState,
  start,
  unread,
} from "./opening-run";
import {
  linesOfFinding,
  orderOf,
  pathOf,
  reviewTab,
  useGoTo,
  useReviewTabState,
  type FindingMark,
} from "./review-tab-state";
import { ranked } from "./fuzzy";
import { DraftCard, FindingCard, LineComposer } from "./lines";
import type { Finding, ReviewPr, ToolDef } from "./model";
import { openKind, openToolTab, openWhenDone } from "./panes";
import { indexOfPlace, linesOfPatch, placeOfLine } from "./patch";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { isWalk, onlyTools, toolsOf } from "./tools";

type Send = ReturnType<typeof useSendFindings>;

const MARKS: readonly {
  mark: FindingMark;
  label: string;
}[] = [
  {
    mark: "expected",
    label: "Expected",
  },
  {
    mark: "not-valid",
    label: "Not valid",
  },
  {
    mark: "commented",
    label: "Comment",
  },
];

const KEYS: readonly (readonly [
  keys: readonly string[],
  does: string,
])[] = [
  [
    [
      "j",
      "k",
    ],
    "Next and previous finding",
  ],
  [
    [
      "]",
      "[",
    ],
    "Next and previous file",
  ],
  [
    [
      "t",
    ],
    "Go to file",
  ],
  [
    [
      "Esc",
    ],
    "Close the peek or the picked lines",
  ],
  [
    [
      "?",
    ],
    "These keys",
  ],
];

/** The pull requests, at a commit, that Review has asked to run their tools on, for the window's life. */
const askedOn = new Set<string>();

/** Where a card sits in the margin, from the top of the grid, and the middle of its first line where the diff holds it. */
type Placed = {
  top: number;
  line: number | undefined;
};

const samePlaces = (a: Readonly<Record<string, Placed>>, b: Readonly<Record<string, Placed>>) =>
  Object.keys(a).length === Object.keys(b).length &&
  Object.entries(a).every(([id, one]) => b[id]?.top === one.top && b[id]?.line === one.line);

export function ReviewTabView({ pr, send }: { pr: ReviewPr; send: Send }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const tools = toolsOf(reviews, pr.manifestId);
  const state = useReviewTabState(pr.key);
  const [reviewing, setReviewing] = useState(false);
  useEffect(
    () => store.readPatches(pr.key),
    [
      store,
      pr.key,
    ],
  );

  const { files, findings } = useMemo(
    () => orderOf(pr, work),
    [
      pr,
      work,
    ],
  );
  const path = pathOf(pr, state.path, files);
  const every = work.every;
  const patch = every.state === "read" ? every.files.find((one) => one.path === path) : undefined;
  const lines = useMemo(
    () => (path === undefined || patch?.patch === undefined ? [] : linesOfPatch(path, patch.patch)),
    [
      path,
      patch,
    ],
  );
  const here = findings.filter((one) => one.at.path === path);
  const tied = useMemo(
    () =>
      new Map(
        here.map((one) => [
          one.id,
          linesOfFinding(lines, one),
        ]),
      ),
    [
      here,
      lines,
    ],
  );
  // Out of the diff first, at the margin's top; the rest by the first line they are about.
  const stacked = [
    ...here,
  ].sort((a, b) => (tied.get(a.id)?.[0] ?? -1) - (tied.get(b.id)?.[0] ?? -1));
  const markOfTool = (finding: Finding) =>
    tools.find((def) => def.id === finding.tool)?.mark ?? "custom";
  const focused = here.find((one) => one.id === state.finding);
  const marked = findings.filter((one) => state.marks[one.id] !== undefined).length;
  const unmarked = marked < findings.length;
  const pick = state.pick;
  const drafts = work.drafts.filter(
    (one) => one.path === path && one.line !== undefined && one.thread === undefined,
  );
  const threads = work.threads.filter((one) => one.path === path);
  const onFile = threads.filter((one) => one.line === undefined);

  // ---- The margin: each card at its line, pushed down past the one above it ----
  const grid = useRef<HTMLDivElement>(null);
  const margin = useRef<HTMLElement>(null);
  const root = useRef<HTMLDivElement>(null);
  const cards = useRef(new Map<string, HTMLDivElement>());
  const anchor = useRef<HTMLSpanElement>(null);
  const [places, setPlaces] = useState<Readonly<Record<string, Placed>>>({});
  const [floor, setFloor] = useState(0);
  const [anchorAt, setAnchorAt] = useState<number | undefined>(undefined);
  const wantedLine = state.wanted?.line === undefined ? -1 : indexOfPlace(lines, state.wanted.line);

  const measure = () => {
    if (grid.current === null || margin.current === null) return;
    const base = grid.current.getBoundingClientRect().top;
    const gap = parseFloat(getComputedStyle(margin.current).rowGap) || 0;
    const textOf = (index: number) =>
      grid.current?.querySelector<HTMLElement>(
        `[data-diff-line="${index}"] > .armada-diff__text`,
      ) ?? null;
    const next: Record<string, Placed> = {};
    let below = 0;
    for (const one of stacked) {
      const first = tied.get(one.id)?.[0];
      const text = first === undefined ? null : textOf(first);
      const box = text?.getBoundingClientRect();
      const lineTop = box === undefined ? 0 : box.top - base;
      const top = Math.max(lineTop, below);
      next[one.id] = {
        top,
        line: box === undefined ? undefined : lineTop + box.height / 2,
      };
      below = top + (cards.current.get(one.id)?.offsetHeight ?? 0) + gap;
    }
    setPlaces((was) => (samePlaces(was, next) ? was : next));
    setFloor(below);
    const wanted = wantedLine < 0 ? null : textOf(wantedLine);
    setAnchorAt(wanted === null ? undefined : wanted.getBoundingClientRect().top - base);
  };
  // Every drawing can move a line: a box opening under one, a card growing a peek. The setters bail where nothing moved.
  useLayoutEffect(measure);
  // A card that grows on its own, as one does when a peek opens in it, draws nothing here, so its size is watched too.
  const latest = useRef(measure);
  latest.current = measure;
  const watch = useRef<ResizeObserver | null>(null);
  watch.current ??= new ResizeObserver(() => latest.current());
  useEffect(() => {
    const watching = watch.current!;
    if (grid.current !== null) watching.observe(grid.current);
    return () => watching.disconnect();
  }, []);

  // ---- Bringing into view what a key, a Jump or Go to file asked for, once its place is measured ----
  const shown = useRef(0);
  useLayoutEffect(() => {
    const wanted = state.wanted;
    if (wanted === undefined || wanted.n === shown.current) return;
    if (wanted.finding !== undefined) {
      const card = cards.current.get(wanted.finding);
      if (card === undefined || places[wanted.finding] === undefined) return;
      shown.current = wanted.n;
      scrollWithin(card, "center");
      card.querySelector<HTMLElement>("article")?.focus({
        preventScroll: true,
      });
      return;
    }
    if (wanted.line !== undefined && wantedLine >= 0) {
      if (anchor.current === null || anchorAt === undefined) return;
      shown.current = wanted.n;
      scrollWithin(anchor.current, "center");
      root.current?.focus({
        preventScroll: true,
      });
      return;
    }
    shown.current = wanted.n;
    const body = root.current?.querySelector<HTMLElement>(".pr-review-tab-view__body");
    if (body !== null && body !== undefined) body.scrollTop = 0;
    root.current?.focus({
      preventScroll: true,
    });
  });

  // ---- Lines and comments ----
  const file: DiffFile | undefined =
    path === undefined
      ? undefined
      : {
          path,
          lines,
        };
  const focusTied = focused === undefined ? [] : (tied.get(focused.id) ?? []);
  const markOf = (_: DiffFile, index: number) => {
    if (pick !== undefined && index >= pick.from && index <= pick.to) return "picked" as const;
    if (focusTied.includes(index))
      return state.marks[focused!.id] === undefined ? ("tied" as const) : ("quiet" as const);
    return undefined;
  };
  const commentedOn = (from: number, to: number) => {
    for (const one of here) {
      const at = tied.get(one.id) ?? [];
      if (at.some((index) => index >= from && index <= to))
        reviewTab.mark(pr.key, one.id, "commented");
    }
  };
  const addDraft = (body: string) => {
    if (pick === undefined || path === undefined) return;
    const at = placeOfLine(lines, pick.to);
    store.addDraft(pr.key, {
      path,
      ...(at === undefined
        ? {}
        : {
            line: at.line,
            side: at.side,
          }),
      body,
    });
    commentedOn(pick.from, pick.to);
    reviewTab.change(pr.key, {
      pick: undefined,
    });
  };
  const under = (_: DiffFile, index: number) => {
    const placed = drafts.filter((one) => {
      const at = indexOfPlace(lines, {
        line: one.line!,
        side: one.side ?? "right",
      });
      return at === index;
    });
    const talk = threads.filter(
      (one) =>
        one.line !== undefined &&
        indexOfPlace(lines, {
          line: one.line,
          side: one.side,
        }) === index,
    );
    const writing = pick !== undefined && pick.to === index;
    if (placed.length === 0 && talk.length === 0 && !writing) return null;
    const pending = work.drafts.length;
    return (
      <div className="pr-under">
        {talk.map((one) => (
          <ThreadHere key={one.id} pr={pr} thread={one} />
        ))}
        {placed.map((one) => (
          <DraftCard key={one.id} draft={one} onRemove={() => store.removeDraft(pr.key, one.id)} />
        ))}
        {writing ? (
          <LineComposer
            keep={`pr-review:${pr.key}:review-tab:${path}:${pick.from}-${pick.to}`}
            label="Comment"
            action={pending === 0 ? "Start a review" : `Add to review (${pending})`}
            {...(pending === 0
              ? {
                  also: {
                    label: "Add single comment",
                    press: (body: string) => {
                      addDraft(body);
                      void store.submit(pr.key, "comment", "");
                    },
                  },
                }
              : {})}
            onAdd={addDraft}
            onCancel={() =>
              reviewTab.change(pr.key, {
                pick: undefined,
              })
            }
          />
        ) : null}
      </div>
    );
  };
  const onLine = {
    label: "Comment on this line. Shift picks the lines to here",
    // Named by its number and what it says, without git's marker, so a screen reader and a walk can tell one line from another.
    name: (_: DiffFile, index: number) => {
      const at = placeOfLine(lines, index);
      const words = lines[index]?.text.slice(1).trim() ?? "";
      const which =
        at === undefined ? "this line" : `${at.side === "left" ? "old line" : "line"} ${at.line}`;
      return words === "" ? `Comment on ${which}` : `Comment on ${which}: ${words}`;
    },
    press: (
      _: DiffFile,
      index: number,
      held: {
        shift: boolean;
      },
    ) =>
      reviewTab.change(pr.key, {
        pick:
          held.shift && pick !== undefined
            ? {
                from: Math.min(pick.from, index),
                to: Math.max(pick.to, index),
              }
            : {
                from: index,
                to: index,
              },
      }),
  };

  const at = path === undefined ? -1 : files.indexOf(path);
  const toFile = (to: string | undefined) => {
    if (to !== undefined) reviewTab.go(pr.key, to);
  };
  const finders = onlyTools(tools).filter((one) => one.agent && !isWalk(one));
  const none =
    pr.page.files.length === 0
      ? "No files changed."
      : finders.some((one) => work.runs[one.id]?.state === "running")
        ? "The tools are running."
        : finders.some((one) => unread(work.runs[one.id], pr.head))
          ? "Not every tool has run on this commit."
          : "No file has findings or open threads.";

  // Opening Review runs the repository's default set where any of it has nothing kept for this commit, asking first unless he said
  // not to. Asked once a commit, so a reopen never asks again.
  const head = headTools(tools);
  const ready = work.runsRead && head.length > 0;
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    if (!ready) return;
    const once = `${pr.key}@${pr.head}`;
    if (askedOn.has(once)) return;
    askedOn.add(once);
    const now = store.get();
    const kept = now.work[pr.key] ?? NO_WORK;
    const chosen = defaultsOf(pr.manifestId, head);
    const due = head.filter((one) => chosen.includes(one.id) && unread(runOf(kept, one), pr.head));
    if (due.length === 0) return;
    // The repository's Ask before running decides, not the head's Don't ask again, so System, Repository can turn it back on.
    if (asksOf(pr.manifestId)) return setAsking(true);
    for (const one of due) start(store, pr, kept, one);
    // Read once the runs are in; what is due is read then, never again for this commit.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    ready,
    pr.key,
    pr.head,
  ]);

  return (
    <div ref={root} className="pr-review-tab-view" tabIndex={-1}>
      <header className="pr-review-tab-view__head">
        <span className="pr-review-tab-view__steps" role="group" aria-label="Files">
          <Button
            variant="secondary"
            size="sm"
            aria-label="Previous file"
            disabled={at <= 0}
            onClick={() => toFile(files[at - 1])}
          >
            Previous
          </Button>
          {at < 0 && path !== undefined ? (
            <Tooltip label="No findings or open threads on this file, so Previous and Next pass it">
              <span className="pr-review-tab-view__of">Not counted</span>
            </Tooltip>
          ) : (
            <span className="pr-review-tab-view__of">
              {files.length === 0 ? "" : `file ${at + 1} of ${files.length}`}
            </span>
          )}
          <Button
            variant="secondary"
            size="sm"
            aria-label="Next file"
            disabled={at >= files.length - 1}
            onClick={() => toFile(files[at + 1])}
          >
            Next
          </Button>
        </span>
        <span className="pr-review-tab-view__path" title={path}>
          {path ?? ""}
        </span>
        {/* Drawn empty where there are no findings, so the columns after it stay where they are. */}
        {findings.length === 0 ? (
          <span className="pr-review-tab-view__marked" />
        ) : (
          <Tooltip label="Findings marked Expected, Not valid or Commented, across every file">
            <span className="pr-review-tab-view__marked">
              {`${marked} of ${findings.length} findings marked`}
            </span>
          </Tooltip>
        )}
        <RunTools pr={pr} />
        <Tooltip label="Keys">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Keys"
            aria-expanded={state.keys}
            onClick={() =>
              reviewTab.change(pr.key, {
                keys: !state.keys,
              })
            }
          >
            <Kbd aria-hidden="true">?</Kbd>
          </Button>
        </Tooltip>
      </header>
      {state.keys ? (
        <dl className="pr-review-tab-view__keys" aria-label="Keys">
          {KEYS.map(([keys, does]) => (
            <div key={does} className="pr-review-tab-view__key">
              <dt>
                {keys.map((one) => (
                  <Kbd key={one}>{one}</Kbd>
                ))}
              </dt>
              <dd>{does}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      <div className="pr-review-tab-view__body">
        <div ref={grid} className="pr-review-tab-view__grid">
          <div
            className="pr-review-tab-view__diff"
            data-tool={focused === undefined ? undefined : markOfTool(focused)}
          >
            {file === undefined ? (
              <p className="pr-review-tab-view__said">{none}</p>
            ) : every.state !== "read" ? (
              <p className="pr-review-tab-view__said">
                {every.state === "said" ? every.said : "Reading the diff."}
              </p>
            ) : lines.length === 0 ? (
              <p className="pr-review-tab-view__said">
                {patch?.said ?? "No lines changed in this file."}
              </p>
            ) : (
              <>
                {/* A thread on the whole file has no line, so it sits over the diff. */}
                {onFile.length === 0 ? null : (
                  <div className="pr-under">
                    {onFile.map((one) => (
                      <ThreadHere key={one.id} pr={pr} thread={one} />
                    ))}
                  </div>
                )}
                <UnifiedDiff
                  bare
                  files={[
                    file,
                  ]}
                  markOf={markOf}
                  under={under}
                  onLine={onLine}
                />
              </>
            )}
          </div>
          <aside
            ref={margin}
            className="pr-review-tab-view__margin"
            aria-label="Findings"
            style={{
              minHeight: floor,
            }}
          >
            {findings.length === 0 ? (
              <p className="pr-review-tab-view__said">
                No findings. A tool's findings are drawn here.
              </p>
            ) : null}
            {stacked.map((one) => (
              <MarginCard
                key={one.id}
                pr={pr}
                finding={one}
                tool={markOfTool(one)}
                send={send}
                mark={state.marks[one.id]}
                focused={one.id === state.finding}
                away={(tied.get(one.id) ?? []).length === 0}
                placed={places[one.id]}
                hold={(element) => {
                  const was = cards.current.get(one.id);
                  if (was !== undefined && was !== element) watch.current?.unobserve(was);
                  if (element === null) {
                    cards.current.delete(one.id);
                    return;
                  }
                  cards.current.set(one.id, element);
                  watch.current?.observe(element);
                }}
                onComment={() => {
                  const at = tied.get(one.id) ?? [];
                  if (at.length === 0) return;
                  reviewTab.change(pr.key, {
                    finding: one.id,
                    pick: {
                      from: at[0]!,
                      to: at.at(-1)!,
                    },
                  });
                }}
              />
            ))}
            {anchorAt === undefined ? null : (
              <span
                ref={anchor}
                className="pr-review-tab-view__anchor"
                aria-hidden
                style={{
                  top: anchorAt,
                }}
              />
            )}
          </aside>
        </div>
      </div>
      {reviewing ? (
        <div className="pr-review-tab-view__review">
          <YourReview
            pr={pr}
            onGo={(one) => {
              const line = placeOfDraft(one);
              reviewTab.go(
                pr.key,
                one.path,
                line === undefined
                  ? {}
                  : {
                      line,
                    },
              );
            }}
            {...(unmarked
              ? {
                  caution: "Some findings are not marked. Submit again to send the review.",
                }
              : {})}
          />
        </div>
      ) : null}
      {asking ? (
        <OpeningAsk
          store={store}
          pr={pr}
          work={work}
          tools={head}
          onClose={() => setAsking(false)}
        />
      ) : null}
      <footer className="pr-review-tab-view__foot">
        <Button
          variant={reviewing ? "ghost" : "secondary"}
          size="sm"
          aria-expanded={reviewing}
          onClick={() => setReviewing((was) => !was)}
        >
          {reviewing ? "Hide review" : "Review changes"}
        </Button>
      </footer>
    </div>
  );
}

/**
 * The explanations and the review tools, one button each in the head, in the order + lists them. A press on a tool runs it, or runs
 * it again, and its findings land beside their lines here. A press on an explanation runs it where nothing is kept, and its tab opens
 * when it is done; one kept opens its tab. Each run asks first, as every run does.
 */
function RunTools({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const tools = headTools(toolsOf(reviews, pr.manifestId));
  if (tools.length === 0) return <span className="pr-review-tab-view__tools" />;
  const press = (tool: ToolDef) => {
    const run = runOf(work, tool);
    if (run?.state === "running") return;
    if (tool.kind === "explanation") {
      if (run !== undefined && run.state !== "idle") return openToolTab(pr.key, tool);
      return store.spend(tool.name, () => {
        openWhenDone(pr.key, tool);
        store.explain(pr.key, tool);
      });
    }
    store.spend(tool.name, () =>
      run === undefined || run.state === "idle"
        ? store.runTool(pr.key, tool)
        : store.rerun(pr.key, tool),
    );
  };
  const nameOf = (tool: ToolDef, run: ReturnType<typeof runState>["run"]) => {
    if (run === "running") return `${tool.name}, running`;
    if (run === "idle") return `Run ${tool.name}`;
    return tool.kind === "explanation" ? `Open ${tool.name}` : `Run ${tool.name} again`;
  };
  return (
    <span className="pr-review-tab-view__tools" role="group" aria-label="Tools">
      {tools.map((tool) => {
        const { run, Glyph, said } = runState(runOf(work, tool), pr.head);
        return (
          <Tooltip key={tool.id} label={`${tool.name}. ${said}`}>
            <Button
              variant="secondary"
              size="sm"
              data-run={run}
              aria-label={nameOf(tool, run)}
              aria-disabled={run === "running" || undefined}
              onClick={() => press(tool)}
            >
              <Glyph size={16} strokeWidth={2} aria-hidden />
              {tool.name}
            </Button>
          </Tooltip>
        );
      })}
    </span>
  );
}

/** A finding in the margin, at its line, with the marks that put it away. */
function MarginCard({
  pr,
  finding,
  tool,
  send,
  mark,
  focused,
  away,
  placed,
  hold,
  onComment,
}: {
  pr: ReviewPr;
  finding: Finding;
  tool: Parameters<typeof FindingCard>[0]["mark"];
  send: Send;
  mark: FindingMark | undefined;
  focused: boolean;
  away: boolean;
  placed: Placed | undefined;
  hold: (element: HTMLDivElement | null) => void;
  onComment: () => void;
}) {
  const top = placed?.top ?? 0;
  return (
    <div
      ref={hold}
      className="pr-review-tab-view__card"
      data-tool={tool}
      data-focused={focused || undefined}
      data-marked={mark !== undefined || undefined}
      data-placed={placed === undefined ? undefined : ""}
      style={{
        top,
      }}
      onFocus={() => {
        if (!focused)
          reviewTab.change(pr.key, {
            finding: finding.id,
          });
      }}
    >
      {placed?.line === undefined ? null : (
        <span
          className="pr-review-tab-view__leader"
          aria-hidden
          style={{
            top: placed.line - top,
            height: Math.max(0, top - placed.line),
          }}
        />
      )}
      <CodeRefs pr={pr}>
        <FindingCard
          finding={finding}
          mark={tool}
          session={send !== undefined}
          onSend={() =>
            send?.([
              finding,
            ])
          }
        >
          {away ? (
            <p className="pr-review-tab-view__away">{`Line ${finding.at.start} is not in the diff.`}</p>
          ) : null}
          <div
            className="pr-review-tab-view__marks"
            role="group"
            aria-label={`Mark ${finding.title}`}
          >
            {/* A line the diff does not hold takes no comment, so it has no Comment; the other two keep their columns. */}
            {MARKS.filter((one) => !(one.mark === "commented" && away)).map((one) => (
              <Button
                key={one.mark}
                variant="ghost"
                size="sm"
                aria-pressed={mark === one.mark}
                onClick={() => {
                  if (mark === one.mark) return reviewTab.mark(pr.key, finding.id, undefined);
                  if (one.mark === "commented") return onComment();
                  reviewTab.mark(pr.key, finding.id, one.mark);
                }}
              >
                {one.label}
              </Button>
            ))}
          </div>
        </FindingCard>
      </CodeRefs>
    </div>
  );
}

/** Go to file, while it is up for this pull request: a few letters of a changed file's path, and the best matches under them. */
export function GoToFile({ pr, onFiles }: { pr: ReviewPr; onFiles: (path: string) => void }) {
  const goTo = useGoTo();
  if (goTo === undefined || goTo.key !== pr.key) return null;
  return createPortal(<GoToBox pr={pr} to={goTo.to} onFiles={onFiles} />, document.body);
}

function GoToBox({
  pr,
  to,
  onFiles,
}: {
  pr: ReviewPr;
  to: "review" | "files";
  onFiles: (path: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [at, setAt] = useState(0);
  const box = useRef<HTMLDivElement>(null);
  const paths = ranked(
    query,
    pr.page.files.map((one) => one.path),
  );
  const pick = (path: string | undefined) => {
    if (path === undefined) return;
    reviewTab.closeGoTo();
    if (to === "files") return onFiles(path);
    reviewTab.go(pr.key, path);
    openKind(pr.key, "pr-review");
  };
  const keys = (event: ReactKeyboardEvent) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setAt((was) =>
        Math.min(
          Math.max(0, paths.length - 1),
          Math.max(0, was + (event.key === "ArrowDown" ? 1 : -1)),
        ),
      );
    } else if (event.key === "Enter") {
      event.preventDefault();
      pick(paths[at]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      reviewTab.closeGoTo();
    }
  };
  return (
    <div
      ref={box}
      className="pr-goto"
      role="dialog"
      aria-label="Go to file"
      onBlur={(event) => {
        if (!box.current?.contains(event.relatedTarget as Node | null)) reviewTab.closeGoTo();
      }}
    >
      <Input
        autoFocus
        mono
        aria-label="File"
        placeholder="Go to file"
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setAt(0);
        }}
        onKeyDown={keys}
      />
      <ul className="pr-goto__rows" role="listbox" aria-label="Changed files">
        {paths.map((path, index) => {
          const cut = path.lastIndexOf("/") + 1;
          return (
            <li
              key={path}
              className="pr-goto__row"
              role="option"
              aria-selected={index === at}
              aria-label={path}
              onMouseDown={(event) => event.preventDefault()}
              onMouseEnter={() => setAt(index)}
              onClick={() => pick(path)}
            >
              <span className="pr-goto__name">{path.slice(cut)}</span>
              <span className="pr-goto__dir">{path.slice(0, cut)}</span>
            </li>
          );
        })}
      </ul>
      {paths.length === 0 ? <p className="pr-goto__none">No changed file matches.</p> : null}
    </div>
  );
}
