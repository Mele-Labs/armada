// What opening the Review tab runs: the tools and explanations of the head row that a repository runs by default, the ask that
// lists them with a box each, and the same boxes in System, Repository. **The default is kept per repository on this machine**,
// under the manifest's id, in Bridge's storage: Fleet has no setting for it. With none kept it is every tool that finds things, no
// explanation, and asking first. A run kept for this commit is never run again by opening.

import { useEffect, useState } from "react";
import { CircleDashed, CircleX, History, LoaderCircle, RefreshCw } from "lucide-react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Checkbox,
  Dialog,
  PrMark,
  SwitchControl,
  agoOf,
  readKept,
  useKept,
  writeKept,
} from "@armada/components";

import type { AgentRun, PrWork, ReviewPr, ToolDef } from "./model";
import { openWhenDone } from "./panes";
import { useReviewStore, useReviews, type ReviewStore } from "./store";
import { isWalk, tilesOf, toolsOf } from "./tools";

/** The head row's tools and explanations, in the order + lists them. The Walk is left out: it finds a place before it runs. */
export const headTools = (defs: readonly ToolDef[]): readonly ToolDef[] =>
  tilesOf(defs).filter((one) => one.agent && !isWalk(one));

export const runOf = (work: PrWork, tool: ToolDef): AgentRun | undefined =>
  tool.kind === "explanation" ? work.explains[tool.id] : work.runs[tool.id];

/** Nothing kept for this commit: never run, or done on an earlier one. */
export const unread = (run: AgentRun | undefined, head: string): boolean =>
  run === undefined || run.state === "idle" || (run.state === "done" && run.head !== head);

const keepOf = (manifestId: string) => `pr-review:run-on-opening:${manifestId}`;

/** A repository's kept choice. `run` absent follows the built-in default, so a tool added later is ticked as the others were. */
type Opening = {
  run?: readonly string[];
  ask: boolean;
};

const NOTHING_KEPT: Opening = {
  ask: true,
};

// An earlier build kept the ticked ids alone.
const revive = (stored: unknown): Opening => {
  if (Array.isArray(stored))
    return {
      run: stored.filter((one): one is string => typeof one === "string"),
      ask: true,
    };
  if (typeof stored !== "object" || stored === null) return NOTHING_KEPT;
  const { run, ask } = stored as {
    run?: unknown;
    ask?: unknown;
  };
  return {
    ...(Array.isArray(run)
      ? {
          run: run.filter((one): one is string => typeof one === "string"),
        }
      : {}),
    ask: ask !== false,
  };
};

const openingOf = (manifestId: string): Opening =>
  readKept(keepOf(manifestId), revive) ?? NOTHING_KEPT;

const keepOpening = (manifestId: string, change: Partial<Opening>) =>
  writeKept(keepOf(manifestId), {
    ...openingOf(manifestId),
    ...change,
  });

const builtIn = (tools: readonly ToolDef[]) =>
  tools.filter((one) => one.kind === "tool").map((one) => one.id);

/** The ids opening Review runs for a repository: what was kept for it, else every tool that finds things. */
export function defaultsOf(manifestId: string, tools: readonly ToolDef[]): readonly string[] {
  return openingOf(manifestId).run ?? builtIn(tools);
}

/** Whether opening Review asks before it runs a repository's default. Asking is on until Don't ask again turns it off. */
export const asksOf = (manifestId: string): boolean => openingOf(manifestId).ask;

/** Starts one, as its button in the head does once asked. An explanation's tab opens when it is done. */
export function start(store: ReviewStore, pr: ReviewPr, work: PrWork, tool: ToolDef): void {
  const done = runOf(work, tool)?.state === "done";
  if (tool.kind === "explanation") {
    openWhenDone(pr.key, tool);
    return done ? store.explainAgain(pr.key, tool) : store.explain(pr.key, tool);
  }
  if (done) store.rerun(pr.key, tool);
  else store.runTool(pr.key, tool);
}

/** What a run is, as its button in the head says it: the glyph, and the words its tooltip reads. */
export function runState(run: AgentRun | undefined, head: string) {
  if (run === undefined || run.state === "idle")
    return {
      run: "idle",
      Glyph: CircleDashed,
      said: "Not run",
    } as const;
  if (run.state === "running")
    return {
      run: "running",
      Glyph: LoaderCircle,
      said: "Running",
    } as const;
  if (run.state === "failed")
    return {
      run: "failed",
      Glyph: CircleX,
      said: `Failed. ${run.said}`,
    } as const;
  const stale = run.head !== head;
  return {
    run: "done",
    Glyph: stale ? RefreshCw : History,
    said: stale
      ? `Done on ${run.head.slice(0, 7)}. The pull request is at ${head.slice(0, 7)}`
      : `Done ${agoOf(new Date(run.at).toISOString())} on ${run.head.slice(0, 7)}`,
  } as const;
}

/**
 * The ask on opening: every tool and explanation of the head row with a box, ticked from the repository's default, and its state
 * beside it. Run starts the ticked ones with nothing kept for this commit. Use these keeps the ticked set as the repository's
 * default; Don't ask again is kept for the repository too, and System, Repository turns it back on.
 */
export function OpeningAsk({
  store,
  pr,
  work,
  tools,
  onClose,
}: {
  store: ReviewStore;
  pr: ReviewPr;
  work: PrWork;
  tools: readonly ToolDef[];
  onClose: () => void;
}) {
  const [ticked, setTicked] = useState<readonly string[]>(() => defaultsOf(pr.manifestId, tools));
  const [keep, setKeep] = useState(false);
  const [again, setAgain] = useState(false);
  const due = tools.filter((one) => ticked.includes(one.id) && unread(runOf(work, one), pr.head));
  const toggle = (id: string) =>
    setTicked((was) =>
      was.includes(id)
        ? was.filter((one) => one !== id)
        : [
            ...was,
            id,
          ],
    );
  return (
    <Dialog
      open
      tone="neutral"
      title="Run on opening"
      confirmLabel="Run"
      confirmDisabled={due.length === 0 && !keep && !again}
      initialFocus="confirm"
      field={
        <div className="pr-opening__keep">
          <Checkbox checked={keep} onChange={(event) => setKeep(event.target.checked)}>
            {`Use these for ${pr.repository} from now on`}
          </Checkbox>
          <Checkbox checked={again} onChange={(event) => setAgain(event.target.checked)}>
            Don't ask again
          </Checkbox>
        </div>
      }
      onConfirm={() => {
        if (keep || again)
          keepOpening(pr.manifestId, {
            ...(keep
              ? {
                  run: tools.filter((one) => ticked.includes(one.id)).map((one) => one.id),
                }
              : {}),
            ...(again
              ? {
                  ask: false,
                }
              : {}),
          });
        onClose();
        // Don't ask again here is the repository's, kept above; the head's buttons go on asking.
        store.spendUnasked(() => {
          for (const one of due) start(store, pr, work, one);
        }, false);
      }}
      onCancel={onClose}
    >
      <p>Running these uses tokens.</p>
      <ul className="pr-opening__tools" aria-label="Tools to run">
        {tools.map((one) => {
          const { Glyph, said } = runState(runOf(work, one), pr.head);
          return (
            <li key={one.id} className="pr-opening__tool">
              <Checkbox checked={ticked.includes(one.id)} onChange={() => toggle(one.id)}>
                {one.name}
              </Checkbox>
              <PrMark Glyph={Glyph} said={said} />
            </li>
          );
        })}
      </ul>
    </Dialog>
  );
}

/**
 * System, Repository's Run on opening: for each repository whose tools Bridge has read, the ask's boxes in head-row order, and
 * Ask before running. Both are what the ask reads and keeps.
 */
export function RunOnOpening({
  repositories,
}: {
  repositories: readonly {
    id: string;
    name: string;
  }[];
}) {
  const store = useReviewStore();
  const reviews = useReviews();
  const ids = repositories.map((one) => one.id).join(" ");
  useEffect(() => {
    for (const one of repositories) store?.readTools(one.id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    store,
    ids,
  ]);
  const read = repositories.flatMap((one) => {
    const tools = headTools(toolsOf(reviews, one.id));
    return tools.length === 0
      ? []
      : [
          {
            ...one,
            tools,
          },
        ];
  });
  if (read.length === 0) return null;
  return (
    <section className="armada-repository-switches" aria-label="Run on opening">
      <h4 className="armada-repository-switches__title" aria-hidden="true">
        Run on opening
      </h4>
      <p className="armada-repository-switches__help">
        What opening a pull request's Review tab runs in each repository. Running these uses tokens.
      </p>
      {read.map((one) => (
        <OpeningDefaults key={one.id} id={one.id} name={one.name} tools={one.tools} />
      ))}
    </section>
  );
}

function OpeningDefaults({
  id,
  name,
  tools,
}: {
  id: string;
  name: string;
  tools: readonly ToolDef[];
}) {
  const [kept, keep] = useKept<Opening>(keepOf(id), NOTHING_KEPT, {
    revive,
  });
  const ticked = kept.run ?? builtIn(tools);
  const toggle = (tool: string, on: boolean) =>
    keep({
      ...kept,
      run: tools
        .filter((one) => (one.id === tool ? on : ticked.includes(one.id)))
        .map((one) => one.id),
    });
  return (
    <Card role="region" aria-label={name} className="pr-opening__repository">
      <CardHeader>
        <CardTitle>{name}</CardTitle>
      </CardHeader>
      <CardContent>
        <ul className="armada-repository-switches__rows">
          <li className="armada-repository-switches__row">
            <span className="armada-repository-switches__name">Ask before running</span>
            <span className="armada-repository-switches__switch">
              <SwitchControl
                size="sm"
                label={`Ask before running in ${name}`}
                checked={kept.ask}
                onChange={(event) =>
                  keep({
                    ...kept,
                    ask: event.target.checked,
                  })
                }
              />
            </span>
          </li>
        </ul>
        <ul className="armada-repository-switches__rows" aria-label={`Run on opening in ${name}`}>
          {tools.map((one) => (
            <li key={one.id} className="armada-repository-switches__row">
              <Checkbox
                checked={ticked.includes(one.id)}
                onChange={(event) => toggle(one.id, event.target.checked)}
              >
                {one.name}
              </Checkbox>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
