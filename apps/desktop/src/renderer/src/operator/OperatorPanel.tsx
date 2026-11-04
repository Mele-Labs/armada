// The Operator panel, in Helm's dock over the whole work, on the panes a Job's area uses (`job-panes/`): tabs and splits he adds,
// closes, moves and resizes, kept as a Job's layout is. Its tabs are Talk, the Log, Waiting to start and Permissions; it opens on
// Talk and the Log in one pane. Drawn only where the window has an Operator store (`store.tsx`), which today is the mock's alone.

import { useState } from "react";
import type { DragEvent, KeyboardEvent, ReactNode } from "react";
import { Bot, Lock } from "lucide-react";
import {
  Button,
  DropdownMenu,
  Input,
  SessionComposer,
  Tooltip,
  type ComposerMode,
  type ComposerTag,
  type DropdownMenuEntry,
} from "@armada/components";
import type { DrawnSketch } from "@armada/screens/src/draft/sessions";
import { askToCloseHelm } from "@armada/shell";

import { select } from "../cockpit/deck/store";
import { searchFiles } from "../commands";
import { JobChip } from "../job-chip";
import { JobPanes } from "../job-panes/JobPanes";
import { focusIn } from "../job-panes/keys";
import { KINDS } from "../job-panes/kinds";
import { operatorPanes } from "../job-panes/store";
import { OPERATOR_TAB_KINDS, addTab, hasKind, type Tab, type TabKind } from "../job-panes/tree";
import { useNumbered } from "../numbered";
import { hostedCommands } from "../session-runner";
import { SketchSheet } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import {
  DID_SAID,
  POSITION_SAID,
  ROLES,
  ageOf,
  orderSaid,
  sourceSaid,
  waitLine,
  waitSaid,
} from "./model";
import type {
  OperatorDid,
  OperatorJob,
  OperatorLineRow,
  OperatorLogRow,
  OperatorPosition,
  OperatorSource,
} from "./model";
import { useOperator } from "./store";
import type { OperatorStore } from "./store";
import "./operator.css";

/** The one owner of the Operator's panes. */
export const OPERATOR = "operator";

export function OperatorPanel() {
  const store = useOperator();
  // Where the Layout control stands: the panel's own bar, since the title bar's slot is the open Job's.
  const [place, setPlace] = useState<HTMLDivElement | null>(null);
  if (store === null) return null;
  return (
    <div className="armada-operator">
      <div className="armada-operator__bar" ref={setPlace} />
      <JobPanes
        jobId={OPERATOR}
        family={operatorPanes}
        controls={place}
        body={(tab) => <Body tab={tab} store={store} />}
        add={{
          entries: addEntries,
          onSelect: (id, pane) => {
            operatorPanes.rearrange(OPERATOR, (now) => addTab(now, pane.id, id as TabKind));
            focusIn(pane.id);
          },
        }}
      />
    </div>
  );
}

/** What + offers: each of the Operator's tabs not open already, since there is one of each. */
function addEntries(): DropdownMenuEntry[] {
  const now = operatorPanes.peek(OPERATOR);
  const shut = OPERATOR_TAB_KINDS.filter((kind) => !hasKind(now, kind));
  return shut.length === 0
    ? [
        {
          kind: "label",
          id: "all-open",
          label: "Every tab is open",
        },
      ]
    : shut.map(
        (kind): DropdownMenuEntry => ({
          kind: "item",
          id: kind,
          label: KINDS[kind].label,
          icon: KINDS[kind].Glyph,
        }),
      );
}

function Body({ tab, store }: { tab: Tab; store: OperatorStore }): ReactNode {
  switch (tab.kind) {
    case "op-talk":
      return <Talk store={store} />;
    case "op-log":
      return (
        <div className="job-pane__scroll armada-operator__scroll">
          <Log store={store} />
        </div>
      );
    case "op-waiting":
      return (
        <div className="job-pane__scroll armada-operator__scroll">
          <Line store={store} />
        </div>
      );
    case "op-permissions":
      return (
        <div className="job-pane__scroll armada-operator__scroll">
          <Permissions store={store} />
        </div>
      );
    default:
      return null;
  }
}

/** Opens a Job from a chip: the Job takes the stage and the dock steps aside for it. */
const openJob = (id: string) => {
  select({
    kind: "session",
    key: `session:${id}`,
  });
  askToCloseHelm();
};

/** A Job a row names, as the Job chip that opens it. */
function NamedJob({ job }: { job: OperatorJob }) {
  return (
    <JobChip
      job={{
        id: job.session,
        title: job.title,
      }}
      onOpen={openJob}
      bare
    />
  );
}

/** What started the item: a Job as its chip, a bug report naming who filed it, or you, Fleet or Waiting to start in words. */
function Source({ from }: { from: OperatorSource }) {
  if (from.kind === "job") return <NamedJob job={from.job} />;
  if (from.kind === "report" && from.by !== "you")
    return (
      <>
        <span className="armada-operator__from">Bug report from</span>
        <NamedJob job={from.by} />
      </>
    );
  return <span className="armada-operator__from">{sourceSaid(from)}</span>;
}

function Line({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const [dragging, setDragging] = useState<string | null>(null);
  const drop = (event: DragEvent<HTMLLIElement>, before: string) => {
    event.preventDefault();
    const id = event.dataTransfer.getData("text/plain") || dragging;
    setDragging(null);
    if (id !== null && id !== "")
      act({
        kind: "move",
        line: id,
        before,
      });
  };
  // ⌥↑ and ⌥↓ move a row as a drag does, so the order is set from the keyboard too.
  const key = (event: KeyboardEvent<HTMLLIElement>, at: number) => {
    if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    const row = state.line[at]!;
    const before =
      event.key === "ArrowUp" ? state.line[at - 1]?.id : (state.line[at + 2]?.id ?? null);
    if (before === undefined || (event.key === "ArrowDown" && at === state.line.length - 1)) return;
    act({
      kind: "move",
      line: row.id,
      before,
    });
  };
  return (
    <ol className="armada-operator__line">
      {state.line.map((row, at) => (
        <li
          key={row.id}
          className="armada-operator__line-row"
          aria-label={row.title}
          aria-description={`Waits for ${waitSaid(row.waits)}. Drag, or ⌥↑ and ⌥↓, to move it`}
          tabIndex={0}
          draggable
          data-dragging={dragging === row.id || undefined}
          onDragStart={(event) => {
            // The row's own drag, not a tab's: the panes' drop targets keep out of it.
            event.stopPropagation();
            event.dataTransfer.setData("text/plain", row.id);
            setDragging(row.id);
          }}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.stopPropagation();
            drop(event, row.id);
          }}
          onDragEnd={() => setDragging(null)}
          onKeyDown={(event) => key(event, at)}
        >
          <LineText row={row} />
          <Button
            size="sm"
            variant="secondary"
            aria-label={`Start now: ${row.title}`}
            onClick={() =>
              act({
                kind: "start",
                line: row.id,
                by: "press",
              })
            }
          >
            Start now
          </Button>
        </li>
      ))}
    </ol>
  );
}

function LineText({ row }: { row: OperatorLineRow }) {
  return (
    <>
      <span className="armada-operator__title">{row.title}</span>
      <span className="armada-operator__wait">{waitLine(row.waits)}</span>
      <span className="armada-operator__named">
        {row.waits.kind === "job" ? <NamedJob job={row.waits.job} /> : null}
      </span>
    </>
  );
}

/** Where an item went, the lanes from left to right. */
const LANES = [
  {
    id: "new",
    said: "New Job",
  },
  {
    id: "existing",
    said: "Existing Job",
  },
  {
    id: "merged",
    said: "Merged",
  },
  {
    id: "in_touch",
    said: "Kept in touch",
  },
  {
    id: "waiting",
    said: "Waiting to start",
  },
  {
    id: "you",
    said: "Sent to you",
  },
  {
    id: "filed",
    said: "Filed",
  },
] as const;
type Lane = (typeof LANES)[number]["id"];

const LANE_OF_DID: Readonly<Record<OperatorDid, Lane>> = {
  started: "new",
  released: "new",
  existing: "existing",
  delegated: "existing",
  answered: "existing",
  returned: "existing",
  resumed: "existing",
  merged: "merged",
  in_touch: "in_touch",
  held: "waiting",
  moved: "waiting",
  raised: "you",
  passed_on: "you",
  filed: "filed",
  overruled: "you",
};

/** The lane an outcome Overrule offers puts an item in. */
function laneOfWay(way: string): Lane {
  if (way.startsWith("New Job")) return "new";
  if (way.startsWith("Merge")) return "merged";
  if (way.startsWith("Keep in touch")) return "in_touch";
  if (way.startsWith("Raise")) return "you";
  return "existing";
}

/** Where the item went: where you overruled it to, else where the ruling sent it. */
const laneOf = (row: OperatorLogRow): Lane =>
  row.changed === undefined ? LANE_OF_DID[row.did] : laneOfWay(row.changed);

/** The Log: the flow of work to where it went, newest at the top. */
function Log({ store }: { store: OperatorStore }) {
  const now = Date.now();
  return (
    <ol className="armada-operator__log">
      <li className="armada-operator__flow-head" role="none">
        <span className="armada-operator__flow-from">Came from</span>
        <span />
        <span>Ruling</span>
        {LANES.map((lane) => (
          <span key={lane.id} className="armada-operator__lane-name">
            {lane.said}
          </span>
        ))}
        <span />
        <span />
        <span />
      </li>
      {store.state.log.map((row) => (
        <LogRow key={row.id} row={row} now={now} store={store} />
      ))}
    </ol>
  );
}

/**
 * One item: what started it, the item, the ruling with why on hover, then where it went, age, Start now and Overrule. Each lane
 * is a column, the Jobs stand in the lane it went to, and a line runs to them from the ruling.
 */
function LogRow({ row, now, store }: { row: OperatorLogRow; now: number; store: OperatorStore }) {
  const line =
    row.line === undefined ? undefined : store.state.line.find((one) => one.id === row.line);
  const named = (row.names ?? []).map((job) => <NamedJob key={job.session} job={job} />);
  const lane = laneOf(row);
  const reached = LANES.findIndex((one) => one.id === lane);
  return (
    <li className="armada-operator__log-row" aria-label={`${sourceSaid(row.from)}: ${row.item}`}>
      <span className="armada-operator__source">
        <Source from={row.from} />
      </span>
      <span className="armada-operator__title">{row.item}</span>
      <Tooltip label={row.why}>
        <span className="armada-operator__did" data-did={row.did} tabIndex={0}>
          {row.changed === undefined ? DID_SAID[row.did] : `Overruled by you: ${row.changed}`}
        </span>
      </Tooltip>
      {LANES.map((one, at) => (
        <span
          key={one.id}
          className="armada-operator__lane"
          data-through={at < reached || undefined}
          data-reached={at === reached || undefined}
        >
          {at !== reached ? null : named.length > 0 ? (
            named
          ) : (
            <span className="armada-operator__lane-mark">{one.said}</span>
          )}
        </span>
      ))}
      <span className="armada-operator__age">{ageOf(row.at, now)}</span>
      <span className="armada-operator__act">
        {line === undefined ? null : (
          <Button
            size="sm"
            variant="secondary"
            aria-label={`Start now: ${line.title}`}
            onClick={() =>
              store.act({
                kind: "start",
                line: line.id,
                by: "press",
              })
            }
          >
            Start now
          </Button>
        )}
      </span>
      <span className="armada-operator__act">
        {row.ways === undefined || row.ways.length === 0 ? null : (
          <DropdownMenu
            triggerLabel="Overrule"
            triggerAriaLabel={`Overrule: ${row.item}`}
            align="end"
            portal
            entries={row.ways.map((way) => ({
              kind: "item" as const,
              id: way,
              label: way,
              ...(row.changed === way
                ? {
                    selected: true,
                  }
                : {}),
            }))}
            onSelect={(way) =>
              store.act({
                kind: "change",
                log: row.id,
                way,
              })
            }
          />
        )}
      </span>
    </li>
  );
}

const POSITIONS: readonly OperatorPosition[] = [
  "on",
  "suggest",
  "off",
];

function Permissions({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  return (
    <>
      <ul className="armada-operator__permissions">
        <li className="armada-operator__permission" data-master="">
          <span className="armada-operator__line-text">
            <span className="armada-operator__title">Operator</span>
            <span className="armada-operator__does">
              Off by default. Off sends everything to you as today
            </span>
          </span>
          <DropdownMenu
            triggerLabel={state.on ? "On" : "Off"}
            triggerAriaLabel={`Operator: ${state.on ? "On" : "Off"}`}
            align="end"
            portal
            entries={[
              {
                kind: "item",
                id: "on",
                label: "On",
                selected: state.on,
              },
              {
                kind: "item",
                id: "off",
                label: "Off",
                selected: !state.on,
              },
            ]}
            onSelect={(id) =>
              act({
                kind: "on",
                on: id === "on",
              })
            }
          />
        </li>
        {ROLES.map(({ role, name, does }) => {
          const position = state.positions[role];
          const differs = state.orders.flatMap((order) =>
            order.kind === "role_off" && order.role === role
              ? [
                  order.repository,
                ]
              : [],
          );
          return (
            <li key={role} className="armada-operator__permission">
              <span className="armada-operator__line-text">
                <span className="armada-operator__title">{name}</span>
                <span className="armada-operator__does">{does}</span>
                {differs.map((repository) => (
                  <span key={repository} className="armada-operator__differs">
                    Off in {repository}
                  </span>
                ))}
              </span>
              <DropdownMenu
                triggerLabel={POSITION_SAID[position]}
                triggerAriaLabel={`${name}: ${POSITION_SAID[position]}`}
                align="end"
                portal
                disabled={!state.on}
                entries={POSITIONS.map((one) => ({
                  kind: "item" as const,
                  id: one,
                  label: POSITION_SAID[one],
                  selected: one === position,
                }))}
                onSelect={(id) =>
                  act({
                    kind: "position",
                    role,
                    position: id as OperatorPosition,
                  })
                }
              />
            </li>
          );
        })}
        <li className="armada-operator__permission">
          <span className="armada-operator__line-text">
            <span className="armada-operator__title">Daily ceiling</span>
            <span className="armada-operator__does">
              Past it, everything passes through as today
            </span>
          </span>
          <span className="armada-operator__ceiling">
            <Input
              type="number"
              min={0}
              aria-label="Daily ceiling"
              unit="calls"
              value={state.ceiling}
              onChange={(event) =>
                act({
                  kind: "ceiling",
                  calls: Number(event.target.value),
                })
              }
            />
          </span>
        </li>
      </ul>
      <p className="armada-operator__lock">
        <Lock size={12} strokeWidth={2} aria-hidden />
        Raise and the destructive lock are always on
      </p>
      <h3 className="armada-operator__subhead">Standing orders</h3>
      <ul className="armada-operator__orders" aria-label="Standing orders">
        {state.orders.map((order) => (
          <li key={order.id} className="armada-operator__permission">
            <span className="armada-operator__order">{orderSaid(order)}</span>
            <Button
              size="sm"
              variant="ghost"
              aria-label={`Remove: ${orderSaid(order)}`}
              onClick={() =>
                act({
                  kind: "unorder",
                  order: order.id,
                })
              }
            >
              Remove
            </Button>
          </li>
        ))}
      </ul>
    </>
  );
}

const NONE: readonly never[] = [];
const NO_SKETCHES: DrawnSketch[] = [];
/** Where a tag stands in what the composer sends. */
const CHIP = "￼";

/** The composer's words with each tag written in by its title, as the conversation shows them. */
const spelled = (text: string, tags: readonly ComposerTag[]): string => {
  let at = 0;
  return text.replaceAll(CHIP, () => `@${tags[at++]?.title ?? ""}`).trim();
};

/** Talk: the conversation over the Job composer, as a Job's thread stands over its box. `@` Jobs and files, `#` issues and pull requests, `/` commands, files and sketches. */
function Talk({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const numbered = useNumbered(undefined);
  const [tuned, setTuned] = useState<{
    model: string | null;
    effort: string | null;
  }>({
    model: null,
    effort: null,
  });
  const [mode, setMode] = useState<ComposerMode>("auto");
  const [drawn, setDrawn] = useState<DrawnSketch[]>(NO_SKETCHES);
  const [padOpen, setPadOpen] = useState(false);
  return (
    <div className="armada-operator__talk-pane">
      <div className="armada-operator__conversation">
        {state.messages.length === 0 ? null : (
          <>
            <span className="armada-operator__clear">
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  act({
                    kind: "clear",
                  })
                }
              >
                Clear
              </Button>
            </span>
            <ol className="armada-operator__talk" role="log" aria-label="Conversation">
              {state.messages.map((message) => (
                <li key={message.id} className="armada-operator__said" data-from={message.from}>
                  {message.from === "operator" ? (
                    <Tooltip label="Operator">
                      <span role="img" aria-label="Operator" className="armada-operator__glyph">
                        <Bot size={12} strokeWidth={2} />
                      </span>
                    </Tooltip>
                  ) : null}
                  <span>{message.text}</span>
                </li>
              ))}
            </ol>
          </>
        )}
      </div>
      <SessionComposer
        label="Message to the Operator"
        working={false}
        mode={mode}
        onMode={setMode}
        model={tuned.model}
        effort={tuned.effort}
        models={draft?.models ?? NONE}
        {...(draft?.defaultModel === undefined
          ? {}
          : {
              defaultModel: draft.defaultModel,
            })}
        efforts={draft?.efforts ?? NONE}
        onTune={setTuned}
        commands={hostedCommands(draft?.commands ?? NONE)}
        taggable={sessions
          .filter((one) => one.dead === undefined && one.title !== undefined)
          .map(
            (one): ComposerTag => ({
              kind: "session",
              id: one.id,
              title: one.title!,
            }),
          )}
        searchFiles={(query) => searchFiles(query)}
        {...(numbered === undefined
          ? {}
          : {
              numbered,
            })}
        tags={NONE}
        onTags={() => undefined}
        draftKey="operator:box"
        drawn={drawn.map(({ id, title }) => ({
          id,
          title,
        }))}
        onDraw={() => setPadOpen(true)}
        onRemoveDrawn={(id) => setDrawn((was) => was.filter((one) => one.id !== id))}
        grows="panel"
        onSend={(sent) => {
          const words = spelled(sent.text, sent.tags);
          const attached = [
            ...sent.files.map((one) => one.name),
            ...drawn.map((one) => one.title),
          ];
          if (words === "" && attached.length === 0) return Promise.resolve(false);
          act({
            kind: "say",
            text: words,
            attached,
          });
          setDrawn(NO_SKETCHES);
          return Promise.resolve(true);
        }}
      />
      <SketchSheet
        open={padOpen}
        onClose={() => setPadOpen(false)}
        onAttach={(sketch) => {
          setDrawn((was) => [
            ...was,
            sketch,
          ]);
          setPadOpen(false);
        }}
      />
    </div>
  );
}
