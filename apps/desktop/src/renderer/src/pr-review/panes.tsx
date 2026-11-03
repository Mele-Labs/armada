// The open pull request's area: its tabs in the panes a Job's are drawn in, of the pull requests' own family (`job-panes/store.ts`),
// so a layout made here never moves a Job's. Overview, Files, Checks, Commits and Review are tabs; each tool and each explanation
// is a tab of its own, opened from + or by its number, and **runs when it is opened and has not run**, asking first as every act that
// spends does. Comments lists the threads, and a press on one brings Files forward at its line.

import { useEffect, useMemo, useState, type ReactNode } from "react";
import { LoaderCircle, SquarePlus } from "lucide-react";
import {
  Button,
  Kbd,
  PrMark,
  PrChecks,
  PrCommits,
  PullRequestFilesView,
  type DropdownMenuEntry,
} from "@armada/components";

import { JobPanes } from "../job-panes/JobPanes";
import { focusIn } from "../job-panes/keys";
import { KINDS } from "../job-panes/kinds";
import { prPanes } from "../job-panes/store";
import {
  activate,
  addTab,
  arrive,
  holderOf,
  panesOf,
  type Arrangement,
  type Pane,
  type Tab,
  type TabKind,
} from "../job-panes/tree";
import { patchesOf } from "../shared-panel/pull-request-patches";

import { useSendFindings } from "./acts";
import { CommentsView } from "./comments";
import { CommentsPane, ThreadHere } from "./comments-pane";
import { Chosen } from "./explain";
import { AddTool } from "./flight";
import { SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, Thread, ToolDef } from "./model";
import { indexOfPlace } from "./patch";
import { Overview } from "./reader";
import { KeptMark, ToolResults } from "./results";
import { NO_WORK, useReviewStore, useReviews, type ReviewStore } from "./store";
import { isWalk, tilesOf, toolsOf } from "./tools";
import { WalkView } from "./walk";

/** The kinds + offers above the tools, in the order the main pane holds them. */
const OFFERED: readonly TabKind[] = [
  "pr-overview",
  "pr-files",
  "pr-checks",
  "pr-commits",
  "pr-review",
  "pr-comments",
];

const WHOSE = {
  armada: "Armada's",
  kit: "the Kit's",
  repository: "the repository's",
} as const;

/** Where a tool is defined, and whose it stands in for, as its row in + says on hover. */
const whenceOf = (tool: ToolDef): string => {
  const source = SOURCE_MARK[tool.source].said;
  return tool.replaces === undefined
    ? source
    : `${source}. Replaces ${WHOSE[tool.replaces]} ${tool.name}`;
};

const tabsOf = (arrangement: Arrangement): Tab[] =>
  panesOf(arrangement.root).flatMap((pane) => pane.tabs);

/** Where a tool's tab opens: the pane that holds tools, else an empty one, as the tools pane is before any tool is opened. */
export const toolsPaneOf = (arrangement: Arrangement): Pane | undefined =>
  holderOf(arrangement, "pr-tool") ??
  panesOf(arrangement.root).find((one) => one.tabs.length === 0);

/** A kind's tab comes to the front where one is open, else opens in `paneId`, or in the main pane, or by the arrival rule. */
export function openKind(key: string, kind: TabKind, paneId?: string): void {
  const arrangement = prPanes.peek(key);
  const open = tabsOf(arrangement).find((one) => one.kind === kind);
  if (open !== undefined) return prPanes.look(key, (now) => activate(now, open.id));
  const main =
    paneId ?? (holderOf(arrangement, "pr-overview") ?? holderOf(arrangement, "pr-review"))?.id;
  if (main !== undefined)
    return prPanes.rearrange(key, (now) => addTab(now, main, kind), undefined, true);
  prPanes.jobOpens(key, kind);
}

/** A tool's tab comes to the front where it is open, else opens in `paneId` or the tools pane. A tool's tab is content, so it leaves the layout as it was. */
export function openToolTab(key: string, tool: ToolDef, paneId?: string): void {
  const arrangement = prPanes.peek(key);
  const open = tabsOf(arrangement).find((one) => one.kind === "pr-tool" && one.page === tool.id);
  if (open !== undefined) return prPanes.look(key, (now) => activate(now, open.id));
  const into = paneId ?? toolsPaneOf(arrangement)?.id;
  if (into !== undefined)
    return prPanes.look(key, (now) => addTab(now, into, "pr-tool", tool.name, tool.id));
  prPanes.rearrange(key, (now) => arrive(now, "pr-tool", tool.name, tool.id), undefined, true);
}

/** Runs a tool or an explanation that has nothing kept for this pull request, asking first. One kept, or running, is only shown. */
export function runIfNotRun(store: ReviewStore, pr: ReviewPr, tool: ToolDef): void {
  const work = store.get().work[pr.key] ?? NO_WORK;
  if (tool.kind === "explanation") {
    const kept = work.explains[tool.id];
    if (kept !== undefined && kept.state !== "idle") return;
    return store.spend(tool.name, () => store.explain(pr.key, tool));
  }
  const kept = work.runs[tool.id];
  // The Walk is found a place before it is written, and one already looking, asking or failed to find shows as it stands.
  const finding = isWalk(tool) && work.walk.find.state !== "idle";
  if (!tool.agent || finding || (kept !== undefined && kept.state !== "idle"))
    store.runTool(pr.key, tool);
  else if (isWalk(tool)) store.spend(tool.name, () => store.findPlace(pr.key));
  else store.spend(tool.name, () => store.runTool(pr.key, tool));
}

const typing = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement &&
  (target.isContentEditable ||
    [
      "INPUT",
      "TEXTAREA",
      "SELECT",
    ].includes(target.tagName));

/** A thread Files was asked to go to. A new one each press, so the same thread asked twice goes there twice. */
type Jump = {
  path: string;
  thread: string;
};

function FilesBody({ pr, jump }: { pr: ReviewPr; jump: Jump | undefined }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  useEffect(
    () => store.readPatches(pr.key),
    [
      store,
      pr.key,
    ],
  );
  const thread = work.threads.find((one) => one.id === jump?.thread);
  const every = work.every;
  const { patches, line } = useMemo(() => {
    const read =
      every.state === "read"
        ? {
            ok: true as const,
            files: every.files,
          }
        : every.state === "said"
          ? {
              ok: false as const,
              said: every.said,
            }
          : undefined;
    const drawn = patchesOf(read);
    if (thread?.line === undefined || drawn.state !== "read")
      return {
        patches: drawn,
        line: undefined,
      };
    const place = {
      line: thread.line,
      side: thread.side,
    };
    const lines = drawn.files.find((one) => one.path === thread.path)?.diff.files[0]?.lines ?? [];
    const at = indexOfPlace(lines, place);
    if (at < 0)
      return {
        patches: drawn,
        line: undefined,
      };
    return {
      patches: patchesOf(read, {
        under: (file, index) =>
          file.path === thread.path && index === at ? (
            <div className="pr-under">
              <ThreadHere pr={pr} thread={thread} />
            </div>
          ) : null,
      }),
      line: at,
    };
  }, [
    every,
    thread,
    pr,
  ]);
  const target = useMemo(
    () =>
      jump === undefined
        ? undefined
        : {
            path: jump.path,
            line,
          },
    [
      jump,
      line,
    ],
  );
  return (
    <div className="job-pane__scroll pr-pane__files">
      <PullRequestFilesView rows={pr.page.files} patches={patches} target={target} />
    </div>
  );
}

function ToolBody({
  pr,
  id,
  send,
  onOpenLink,
}: {
  pr: ReviewPr;
  id: string | undefined;
  send: ReturnType<typeof useSendFindings>;
  onOpenLink: (address: string) => void;
}) {
  const store = useReviewStore()!;
  const tool = toolsOf(useReviews(), pr.manifestId).find((one) => one.id === id);
  if (tool === undefined) return null;
  const run = () => runIfNotRun(store, pr, tool);
  return (
    <div className="job-pane__scroll">
      {tool.kind === "explanation" ? (
        <Chosen pr={pr} def={tool} send={send} onOpenLink={onOpenLink} onRun={run} />
      ) : isWalk(tool) ? (
        <WalkView pr={pr} tool={tool} back={false} onRun={run} />
      ) : (
        <ToolResults pr={pr} tool={tool} send={send} back={false} onRun={run} />
      )}
    </div>
  );
}

/** What a pull request's pane with no tab draws: every explanation and tool, as + lists them, and Add a tool under them. A row opens its tool's tab here. */
function ToolList({
  pr,
  tools,
  onPress,
  onAdd,
}: {
  pr: ReviewPr;
  tools: readonly ToolDef[];
  onPress: (tool: ToolDef) => void;
  onAdd: () => void;
}) {
  const work = useReviews().work[pr.key] ?? NO_WORK;
  return (
    <div className="job-pane__scroll pr-tool-list">
      <ul className="pr-tool-list__rows" aria-label="Tools">
        {tools.map((tool, at) => {
          const run = tool.kind === "explanation" ? work.explains[tool.id] : work.runs[tool.id];
          const { Glyph } = TOOL_MARK[tool.mark];
          return (
            <li key={tool.id}>
              <button
                type="button"
                className="pr-tool-list__row"
                data-tool={tool.mark}
                aria-label={tool.name}
                onClick={() => onPress(tool)}
              >
                <span className="pr-tool-list__glyph">
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                </span>
                <span className="pr-tool-list__name">{tool.name}</span>
                <span className="pr-tool-list__key">
                  {at < 9 ? <Kbd aria-hidden="true">{at + 1}</Kbd> : null}
                </span>
                <span className="pr-tool-list__mark">
                  {run?.state === "running" ? (
                    <PrMark Glyph={LoaderCircle} said="Running" tone="running" />
                  ) : (
                    <KeptMark run={run} head={pr.head} />
                  )}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      <div className="pr-tool-list__add">
        <Button variant="ghost" size="sm" onClick={onAdd}>
          <SquarePlus size={16} strokeWidth={2} aria-hidden />
          Add a tool
        </Button>
      </div>
    </div>
  );
}

/** The pull request's panes. `controls` is where its Layout control stands, in the reader's head. */
export function PrPanes({
  pr,
  onOpenLink,
  controls,
}: {
  pr: ReviewPr;
  onOpenLink: (address: string) => void;
  controls: Element | null;
}) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const send = useSendFindings(pr);
  const tools = tilesOf(toolsOf(reviews, pr.manifestId));
  const [adding, setAdding] = useState(false);
  const [jump, setJump] = useState<Jump | undefined>(undefined);
  const { sheet } = reviews;

  /** A tool from + or its number: its tab, and a run where nothing is kept for it. */
  const press = (tool: ToolDef, paneId?: string) => {
    openToolTab(pr.key, tool, paneId);
    runIfNotRun(store, pr, tool);
  };

  // The keys 1 to 9 open the tools in the order + lists them, and the one after the last adds a tool, where no box is being typed in.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey ||
        typing(event.target) ||
        adding ||
        sheet !== undefined ||
        (event.target instanceof Element && event.target.closest('[role="menu"]') !== null) ||
        document.querySelector('[role="dialog"], [role="alertdialog"]') !== null
      )
        return;
      const at = Number(event.key) - 1;
      if (!Number.isInteger(at) || at < 0 || at > 8) return;
      if (at < tools.length) press(tools[at]!);
      else if (at === tools.length) setAdding(true);
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  });

  const go = (thread: Thread) => {
    setJump({
      path: thread.path,
      thread: thread.id,
    });
    openKind(pr.key, "pr-files");
  };

  const entries = (): DropdownMenuEntry[] => [
    ...OFFERED.map(
      (kind): DropdownMenuEntry => ({
        kind: "item",
        id: `kind:${kind}`,
        label: KINDS[kind].label,
        icon: KINDS[kind].Glyph,
      }),
    ),
    {
      kind: "separator",
      id: "tools-rule",
    },
    ...tools.map((tool, at): DropdownMenuEntry => {
      const run = tool.kind === "explanation" ? work.explains[tool.id] : work.runs[tool.id];
      return {
        kind: "item",
        id: `tool:${tool.id}`,
        label: tool.name,
        icon: TOOL_MARK[tool.mark].Glyph,
        hint: whenceOf(tool),
        ...(at < 9
          ? {
              shortcut: String(at + 1),
            }
          : {}),
        ...(run?.state === "running"
          ? {
              state: "running" as const,
            }
          : {}),
      };
    }),
    {
      kind: "item",
      id: "add-tool",
      label: "Add a tool",
      icon: SquarePlus,
      act: true,
      ...(tools.length < 9
        ? {
            shortcut: String(tools.length + 1),
          }
        : {}),
    },
  ];

  const choose = (id: string, pane: Pane) => {
    if (id === "add-tool") return setAdding(true);
    if (id.startsWith("kind:")) {
      openKind(pr.key, id.slice("kind:".length) as TabKind, pane.id);
      return focusIn(prPanes.peek(pr.key).focus);
    }
    const tool = tools.find((one) => `tool:${one.id}` === id);
    if (tool === undefined) return;
    press(tool, pane.id);
    focusIn(prPanes.peek(pr.key).focus);
  };

  const bodyOf = (tab: Tab): ReactNode => {
    switch (tab.kind) {
      case "pr-overview":
        return (
          <div className="job-pane__scroll">
            <Overview pr={pr} send={send} onOpenLink={onOpenLink} explanations={false} />
          </div>
        );
      case "pr-files":
        return <FilesBody pr={pr} jump={jump} />;
      case "pr-checks":
        return (
          <div className="job-pane__scroll">
            <PrChecks rows={pr.page.checks} onOpenLink={onOpenLink} />
          </div>
        );
      case "pr-commits":
        return (
          <div className="job-pane__scroll">
            <PrCommits rows={pr.page.commits} />
          </div>
        );
      case "pr-review":
        return (
          <div className="job-pane__scroll">
            <CommentsView pr={pr} send={send} />
          </div>
        );
      case "pr-comments":
        return (
          <div className="job-pane__scroll">
            <CommentsPane pr={pr} onGo={go} />
          </div>
        );
      case "pr-tool":
        return <ToolBody pr={pr} id={tab.page} send={send} onOpenLink={onOpenLink} />;
      default:
        return null;
    }
  };

  return (
    <>
      <JobPanes
        jobId={pr.key}
        family={prPanes}
        controls={controls}
        body={bodyOf}
        add={{
          entries,
          onSelect: choose,
        }}
        empty={(pane) => (
          <ToolList
            pr={pr}
            tools={tools}
            onPress={(tool) => press(tool, pane.id)}
            onAdd={() => setAdding(true)}
          />
        )}
        glyphOf={(tab) => {
          if (tab.kind !== "pr-tool") return undefined;
          const tool = tools.find((one) => one.id === tab.page);
          return tool === undefined ? undefined : TOOL_MARK[tool.mark].Glyph;
        }}
      />
      <AddTool open={adding} pr={pr} onClose={() => setAdding(false)} />
    </>
  );
}
