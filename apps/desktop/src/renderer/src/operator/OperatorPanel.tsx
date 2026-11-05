// The Operator panel, in the dock over the whole work, on the panes a Job's area uses (`job-panes/`): tabs and splits he adds,
// closes, moves and resizes, kept as a Job's layout is. Its tabs are Talk, the Log, Waiting to start and Permissions; it opens on
// Talk and the Log in one pane. Drawn only where the window has an Operator store (`store.tsx`), which today is the mock's alone.

import { createContext, useContext, useState } from "react";
import type { DragEvent, KeyboardEvent, ReactNode } from "react";
import { Bot, Check, Clock, GripVertical, LoaderCircle, Lock, MessagesSquare } from "lucide-react";
import {
  Button,
  DropdownMenu,
  Input,
  SessionComposer,
  SplitButton,
  Tooltip,
  YourMessage,
  type ComposerMode,
  type ComposerTag,
  type DropdownMenuEntry,
} from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { DrawnSketch } from "@armada/screens/src/draft/sessions";
import { askToCloseOperator, jobIdOf } from "@armada/shell";

import { select } from "../cockpit/deck/store";
import { searchFiles } from "../commands";
import { JobChip } from "../job-chip";
import { JobPanes } from "../job-panes/JobPanes";
import { focusIn } from "../job-panes/keys";
import { KINDS } from "../job-panes/kinds";
import { operatorPanes } from "../job-panes/store";
import {
  OPERATOR_TAB_KINDS,
  activate,
  addTab,
  hasKind,
  holderOf,
  panesOf,
  type Tab,
  type TabKind,
} from "../job-panes/tree";
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
  outcomeOf,
  sourceSaid,
  turnLine,
  turnTag,
  waitLine,
  waitSaid,
} from "./model";
import type {
  OperatorDid,
  OperatorJob,
  OperatorLineRow,
  OperatorLogRow,
  OperatorPosition,
  OperatorRaise,
  OperatorSource,
  StandingOrder,
  OperatorTurn,
} from "./model";
import { takeNamed } from "./takes/take";
import { useOperator } from "./store";
import type { OperatorStore } from "./store";
import "./operator.css";

/** The one owner of the Operator's panes. */
export const OPERATOR = "operator";

/**
 * Whether a row belongs under the title row's picker. The picker narrows what is shown and nothing else: the Operator sees every
 * Job on the Mac whichever is picked. A row that names no repository is shown under every one.
 */
const Picked = createContext<RepositorySummary | null>(null);

function usePickedFilter(): (repository: string | undefined) => boolean {
  const picked = useContext(Picked);
  const id = picked === null ? undefined : jobIdOf(picked);
  return (repository) => id === undefined || repository === undefined || repository === id;
}

/** `place` is where the Layout control stands: the dock's head beside Close, since the title bar's slot is the open Job's. */
export function OperatorPanel({ picked }: { picked: RepositorySummary | null }) {
  const store = useOperator();
  // Where the Layout control stands: the panel's own bar, since the title bar's slot is the open Job's.
  const [place, setPlace] = useState<HTMLDivElement | null>(null);
  if (store === null) return null;
  return (
    <Picked.Provider value={picked}>
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
    </Picked.Provider>
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
  const pickedTake = takeNamed(store.state.take);
  if (pickedTake?.Talk !== undefined && tab.kind === "op-talk")
    return <pickedTake.Talk store={store} />;
  if (pickedTake !== undefined && tab.kind === "op-queue")
    return (
      <div className="job-pane__scroll armada-operator__scroll">
        <pickedTake.Queue store={store} />
      </div>
    );
  switch (tab.kind) {
    case "op-talk":
      return <Talk store={store} />;
    case "op-queue":
      return (
        <div className="job-pane__scroll armada-operator__scroll">
          <Queue store={store} />
        </div>
      );
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
  askToCloseOperator();
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
  const shown = usePickedFilter();
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
      {state.line.map((row, at) =>
        !shown(row.repository) ? null : (
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
        ),
      )}
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
  const shown = usePickedFilter();
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
      {store.state.log
        .filter((row) => shown(row.repository))
        .map((row) => (
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
          const ordered = state.orders.flatMap((order) =>
            order.kind === "role_off" && order.role === role
              ? [
                  order.repository,
                ]
              : [],
          );
          const set = (state.repositories ?? []).flatMap((one) => {
            const there = one.positions[role];
            return there === undefined
              ? []
              : [
                  `${POSITION_SAID[there]} in ${one.name}`,
                ];
          });
          return (
            <li key={role} className="armada-operator__permission">
              <span className="armada-operator__line-text">
                <span className="armada-operator__title">{name}</span>
                <span className="armada-operator__does">{does}</span>
                {ordered.map((repository) => (
                  <span key={repository} className="armada-operator__differs">
                    Off in {repository} by standing order
                  </span>
                ))}
                {set.map((said) => (
                  <span key={said} className="armada-operator__differs">
                    {said}
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
        {state.model === undefined ? null : (
          <li className="armada-operator__permission">
            <span className="armada-operator__line-text">
              <span className="armada-operator__title">Model</span>
              <span className="armada-operator__does">Also in Settings, System models</span>
            </span>
            <DropdownMenu
              triggerLabel={state.model}
              triggerAriaLabel={`Model: ${state.model}`}
              align="end"
              portal
              entries={(state.models ?? []).map((one) => ({
                kind: "item" as const,
                id: one,
                label: one,
                selected: one === state.model,
              }))}
              onSelect={(id) =>
                act({
                  kind: "model",
                  model: id,
                })
              }
            />
          </li>
        )}
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
      <Repositories store={store} />
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

/** Each repository whose roles differ from the machine's, a column a role, and a menu to add another. Drawn only where the window holds repositories. */
function Repositories({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  if (state.repositories === undefined) return null;
  const listed = state.repositories.map((one) => one.name);
  const others = (state.served ?? []).filter((one) => !listed.includes(one));
  return (
    <>
      <h3 className="armada-operator__subhead">By repository</h3>
      <ul className="armada-operator__repositories" aria-label="By repository">
        <li className="armada-operator__repository-head" role="none">
          <span />
          {ROLES.map(({ role, name }) => (
            <span key={role}>{name}</span>
          ))}
        </li>
        {state.repositories.map((one) => (
          <li key={one.name} className="armada-operator__repository" aria-label={one.name}>
            <span className="armada-operator__title">{one.name}</span>
            {ROLES.map(({ role, name }) => {
              const there = one.positions[role];
              const said = there === undefined ? "Machine" : POSITION_SAID[there];
              return (
                <DropdownMenu
                  key={role}
                  triggerLabel={said}
                  triggerAriaLabel={`${name} in ${one.name}: ${said}`}
                  align="end"
                  portal
                  disabled={!state.on}
                  entries={[
                    {
                      kind: "item" as const,
                      id: "machine",
                      label: `Machine: ${POSITION_SAID[state.positions[role]]}`,
                      selected: there === undefined,
                    },
                    ...POSITIONS.map((position) => ({
                      kind: "item" as const,
                      id: position,
                      label: POSITION_SAID[position],
                      selected: position === there,
                    })),
                  ]}
                  onSelect={(id) =>
                    act({
                      kind: "repository",
                      name: one.name,
                      role,
                      position: id === "machine" ? null : (id as OperatorPosition),
                    })
                  }
                />
              );
            })}
          </li>
        ))}
      </ul>
      {others.length === 0 ? null : (
        <div className="armada-operator__add-repository">
          <DropdownMenu
            triggerLabel="Add repository"
            align="start"
            portal
            entries={others.map((one) => ({
              kind: "item" as const,
              id: one,
              label: one,
            }))}
            onSelect={(id) =>
              act({
                kind: "add-repository",
                name: id,
              })
            }
          />
        </div>
      )}
    </>
  );
}

/** Talk in front, where it is open, and open where it is not: pressing a row works the item through there. */
function showTalk() {
  operatorPanes.rearrange(OPERATOR, (now) => {
    const pane = holderOf(now, "op-talk");
    const tab = pane?.tabs.find((one) => one.kind === "op-talk");
    if (tab !== undefined) return activate(now, tab.id);
    const first = panesOf(now.root)[0];
    return first === undefined ? now : addTab(now, first.id, "op-talk");
  });
}

/** A row that drags, and moves on ⌥↑ and ⌥↓. A drop lands above or below the row by which half the pointer is over. */
function useRowDrag(ids: readonly string[], move: (id: string, before: string | null) => void) {
  const [dragging, setDragging] = useState<string | null>(null);
  const [over, setOver] = useState<{
    id: string;
    side: "above" | "below";
  } | null>(null);
  const sideOf = (event: DragEvent<HTMLLIElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    return event.clientY > box.top + box.height / 2 ? "below" : "above";
  };
  return {
    over,
    props: (id: string, movable: boolean) => ({
      tabIndex: 0,
      draggable: movable,
      "data-dragging": dragging === id || undefined,
      "data-over": over?.id === id ? over.side : undefined,
      onDragStart: (event: DragEvent<HTMLLIElement>) => {
        // The row's own drag, not a tab's: the panes' drop targets keep out of it.
        event.stopPropagation();
        event.dataTransfer.setData("text/plain", id);
        setDragging(id);
      },
      onDragOver: (event: DragEvent<HTMLLIElement>) => {
        event.preventDefault();
        setOver({
          id,
          side: sideOf(event),
        });
      },
      onDrop: (event: DragEvent<HTMLLIElement>) => {
        event.preventDefault();
        event.stopPropagation();
        const moving = event.dataTransfer.getData("text/plain") || dragging;
        const after = ids[ids.indexOf(id) + 1] ?? null;
        setDragging(null);
        setOver(null);
        if (moving) move(moving, sideOf(event) === "above" ? id : after);
      },
      onDragEnd: () => {
        setDragging(null);
        setOver(null);
      },
      onKeyDown: (event: KeyboardEvent<HTMLLIElement>) => {
        if (!movable || !event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown"))
          return;
        event.preventDefault();
        const at = ids.indexOf(id);
        const before = event.key === "ArrowUp" ? ids[at - 1] : (ids[at + 2] ?? null);
        if (before !== undefined && (event.key === "ArrowUp" || at < ids.length - 1))
          move(id, before);
      },
    }),
  };
}

/** What the Operator is working through, in his order, and what it waits on him for. Each list is drawn only while it holds something. */
function Queue({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const shown = usePickedFilter();
  const all = state.turns ?? NONE_YET;
  const turns = all.filter((one) => one.message !== undefined || shown(one.repository));
  const raises = (state.raises ?? []).filter((one) => shown(one.manifest));
  const turnDrag = useRowDrag(
    turns.filter((one) => one.done === undefined).map((one) => one.id),
    (turn, before) =>
      act({
        kind: "move-turn",
        turn,
        before,
      }),
  );
  const busy = all.some((one) => one.done === undefined);
  const discuss = (raise: string, now: boolean) => {
    act({
      kind: "discuss",
      raise,
      now,
    });
    if (now || !busy) showTalk();
  };
  const raiseDrag = useRowDrag(
    raises.map((one) => one.id),
    (raise, before) =>
      act({
        kind: "move-raise",
        raise,
        before,
      }),
  );
  return (
    <div className="armada-operator__queue">
      {turns.length === 0 ? null : (
        <section aria-label="Operator's Queue">
          <h3 className="armada-operator__queue-head">Operator's Queue</h3>
          <ol className="armada-operator__queue-rows" data-turns="">
            {turns.map((one) => {
              const at = all.indexOf(one);
              const reading = at === 0 && one.done === undefined;
              const said = one.done === undefined ? (reading ? READING : QUEUED) : one.done;
              const Glyph = one.done !== undefined ? Check : reading ? LoaderCircle : Clock;
              const line = turnLine(all, at);
              const repository = one.start?.repository ?? one.repository;
              return (
                <li
                  key={one.id}
                  className="armada-operator__queue-row"
                  aria-label={one.text}
                  aria-description="Drag, or ⌥↑ and ⌥↓, to change what the Operator takes next"
                  data-reading={reading || undefined}
                  data-done={one.done !== undefined || undefined}
                  {...turnDrag.props(one.id, !reading && one.done === undefined)}
                >
                  <span className="armada-operator__grip">
                    {reading || one.done !== undefined ? null : (
                      <GripVertical size={12} strokeWidth={2} aria-hidden />
                    )}
                  </span>
                  <Tooltip label={said}>
                    <span role="img" aria-label={said} className="armada-operator__glyph">
                      <Glyph size={12} strokeWidth={2} aria-hidden />
                    </span>
                  </Tooltip>
                  <span className="armada-operator__raise">
                    <span className="armada-operator__row-head">
                      <span className="armada-operator__tag">{turnTag(one)}</span>
                      {repository === undefined ? null : (
                        <span className="armada-operator__repo">{repository}</span>
                      )}
                    </span>
                    <span className="armada-operator__row-title">{one.text}</span>
                    {line === undefined ? null : (
                      <span
                        className="armada-operator__raise-read"
                        data-put={one.put !== undefined || undefined}
                      >
                        {line}
                      </span>
                    )}
                  </span>
                  {one.discuss === true && one.raise !== undefined ? (
                    <Button
                      size="sm"
                      variant="secondary"
                      aria-label={`Discuss now: ${one.text}`}
                      onClick={() => discuss(one.raise!, true)}
                    >
                      Discuss now
                    </Button>
                  ) : one.start === undefined || one.done !== undefined ? (
                    <span />
                  ) : (
                    <Button
                      size="sm"
                      variant="secondary"
                      aria-label={`Start without waiting: ${one.text}`}
                      onClick={() =>
                        act({
                          kind: "start-turn",
                          turn: one.id,
                        })
                      }
                    >
                      Start without waiting
                    </Button>
                  )}
                </li>
              );
            })}
          </ol>
        </section>
      )}
      {raises.length === 0 ? null : (
        <section aria-label="Waiting on you">
          <h3 className="armada-operator__queue-head">Waiting on you</h3>
          <ol className="armada-operator__queue-rows" data-ways="">
            {raises.map((one) => {
              const said =
                one.settled ??
                (one.suggests ? "What the Operator would do" : "Raised by the Operator");
              const Glyph = one.settled === undefined ? Bot : Check;
              return (
                <li
                  key={one.id}
                  className="armada-operator__queue-row"
                  aria-label={one.item}
                  data-open={state.open === one.id || undefined}
                  {...raiseDrag.props(one.id, one.settled === undefined)}
                >
                  <span className="armada-operator__grip">
                    {one.settled === undefined ? (
                      <GripVertical size={12} strokeWidth={2} aria-hidden />
                    ) : null}
                  </span>
                  <Tooltip label={said}>
                    <span role="img" aria-label={said} className="armada-operator__glyph">
                      <Glyph size={12} strokeWidth={2} aria-hidden />
                    </span>
                  </Tooltip>
                  <span className="armada-operator__raise">
                    <span className="armada-operator__row-head">
                      {one.tag === undefined ? null : (
                        <span className="armada-operator__tag">{one.tag}</span>
                      )}
                      <span className="armada-operator__repo">{one.repository}</span>
                    </span>
                    <span className="armada-operator__row-title">{one.item}</span>
                    <span className="armada-operator__raise-read">
                      {one.settled ??
                        (one.toDiscuss === true
                          ? "In the Operator's Queue. It brings this up in Discussion when it gets there"
                          : one.read)}
                    </span>
                  </span>
                  <RaiseActs store={store} raise={one} discuss={discuss} busy={busy} />
                </li>
              );
            })}
          </ol>
        </section>
      )}
    </div>
  );
}

/**
 * An item's acts as one split button: the Operator's pick on the face, the rest behind the caret. Settled, it is Open Job where it
 * names one. The same control on its row and on its card.
 */
function RaiseActs({
  store,
  raise,
  discuss,
  busy,
}: {
  store: OperatorStore;
  raise: OperatorRaise;
  discuss?: (raise: string, now: boolean) => void;
  busy?: boolean;
}) {
  const { state, act } = store;
  if (raise.settled !== undefined)
    return raise.job === undefined ? (
      <span />
    ) : (
      <Button
        size="sm"
        variant="secondary"
        aria-label={`Open Job: ${raise.job.title}`}
        onClick={() => openJob(raise.job!.session)}
      >
        Open Job
      </Button>
    );
  const [face, ...rest] = raise.ways;
  const now = raise.toDiscuss === true || busy !== true;
  const pressed = (way: string) =>
    act({
      kind: "raise",
      raise: raise.id,
      way,
    });
  const items = [
    ...(discuss === undefined || state.open === raise.id
      ? []
      : [
          {
            label: now ? "Discuss now" : "Discuss when ready",
            note: now
              ? "Brings it up in Discussion now"
              : "Joins the Operator's Queue, and comes up in Discussion when it gets there",
            onSelect: () => discuss(raise.id, now),
          },
        ]),
    ...(raise.job === undefined
      ? []
      : [
          {
            label: "Open Job",
            onSelect: () => openJob(raise.job!.session),
          },
        ]),
    ...rest.map((way) => ({
      label: way,
      onSelect: () => pressed(way),
    })),
  ];
  if (face === undefined) return <span />;
  return (
    <SplitButton
      size="sm"
      items={items}
      menuLabel={`More for: ${raise.item}`}
      faceLabel={`${face}: ${raise.item}`}
      onAction={() => pressed(face)}
    >
      {face}
    </SplitButton>
  );
}

/** The item being discussed, pinned above the conversation until it is settled or put back with Not now, so talk never buries it. */
function ItemCard({ store, raise }: { store: OperatorStore; raise: OperatorRaise }) {
  return (
    <div className="armada-operator__card" role="group" aria-label={`Item: ${raise.item}`}>
      <span className="armada-operator__card-band">
        <span>Discussing</span>
        {raise.tag === undefined ? null : <span>{raise.tag}</span>}
        <span className="armada-operator__repo">{raise.repository}</span>
      </span>
      <span className="armada-operator__card-body">
        <span className="armada-operator__row-title">{raise.item}</span>
        <span className="armada-operator__raise-read">{raise.settled ?? raise.read}</span>
        <span className="armada-operator__presses">
          <RaiseActs store={store} raise={raise} />
          {raise.settled !== undefined ? null : (
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                store.act({
                  kind: "not-now",
                })
              }
            >
              Not now
            </Button>
          )}
        </span>
      </span>
    </div>
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

/** The order words came to, in the form Fleet enforces. It stands for nothing until Keep. */
function OrderProposed({
  order,
  settled,
  onSettle,
}: {
  order: StandingOrder;
  settled: "kept" | "left" | undefined;
  onSettle: (keep: boolean) => void;
}) {
  const said = orderSaid(order);
  return (
    <span className="armada-operator__proposed" role="group" aria-label={`Standing order: ${said}`}>
      <span className="armada-operator__order">{said}</span>
      {settled === undefined ? (
        <>
          <Button size="sm" variant="secondary" onClick={() => onSettle(true)}>
            Keep
          </Button>
          <Button size="sm" variant="ghost" onClick={() => onSettle(false)}>
            Leave it
          </Button>
        </>
      ) : (
        <span className="armada-operator__does">{settled === "kept" ? "Kept" : "Left"}</span>
      )}
    </span>
  );
}

const NONE_YET: readonly OperatorTurn[] = [];
const READING = "The Operator is reading this";
const QUEUED = "Queued. The Operator gets to it in turn";

/** Where an item came up in the conversation: a marker, since the card itself stays pinned above. */
function CardMarker({ store, id, ends }: { store: OperatorStore; id: string; ends: boolean }) {
  const found = store.state.raises?.find((one) => one.id === id);
  if (found === undefined) return null;
  // Where it came up says so; only the mark where the talk ended says what came of it.
  const settled = ends ? found.settled : undefined;
  const raise = found;
  const outcome = settled === undefined ? undefined : outcomeOf(settled);
  return (
    <span
      className="armada-operator__marker"
      data-settled={outcome === undefined ? undefined : outcome.letGo ? "let-go" : "done"}
    >
      <span className="armada-operator__marker-head">
        {outcome === undefined ? (
          <MessagesSquare size={12} strokeWidth={2} aria-hidden />
        ) : (
          <Check size={12} strokeWidth={2} aria-hidden />
        )}
        <span className="armada-operator__marker-what">
          {outcome === undefined ? "Brought up" : outcome.label}
        </span>
        <span className="armada-operator__marker-item">{raise.item}</span>
      </span>
      {outcome?.what === undefined ? null : (
        <span className="armada-operator__marker-result">{outcome.what}</span>
      )}
    </span>
  );
}

/** What the Operator says before anything is said: what it is on, what it takes next, and the first thing that needs you. Read off the
    board each time, so it is never stale; nothing on the board, nothing is said. */
function openingOf(store: OperatorStore): string | null {
  const open = (store.state.turns ?? []).filter((one) => one.done === undefined);
  const first = (store.state.raises ?? []).find((one) => one.settled === undefined);
  const said = [
    open[0] === undefined ? null : `I'm on ${open[0].text}.`,
    open[1] === undefined ? null : `Next is ${open[1].text}.`,
    first === undefined ? null : `First for you: ${first.item}, under Needs you.`,
  ].filter((one) => one !== null);
  return said.length === 0 ? null : said.join(" ");
}

function Opening({ store }: { store: OperatorStore }) {
  const said = openingOf(store);
  if (said === null) return null;
  return (
    <ol className="armada-operator__talk" role="log" aria-label="Conversation">
      <li className="armada-helm-thread__row" data-actor="helm" data-from="agent">
        <div className="armada-helm-thread__head">
          <span className="armada-helm-thread__who">Operator</span>
        </div>
        <div className="armada-helm-thread__message">
          <p className="armada-session-said">{said}</p>
        </div>
      </li>
    </ol>
  );
}

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
  const item = state.raises?.find((one) => one.id === state.open);
  const messages = state.messages;
  return (
    <div className="armada-operator__talk-pane">
      {item === undefined ? null : <ItemCard store={store} raise={item} />}
      <div className="armada-operator__conversation">
        {messages.length === 0 ? (
          <Opening store={store} />
        ) : (
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
              {messages.map((message) => {
                if (message.card !== undefined)
                  return (
                    <li key={message.id} className="armada-operator__said">
                      <CardMarker store={store} id={message.card} ends={message.ends === true} />
                    </li>
                  );
                if (message.from === "you") {
                  const at = state.turns?.findIndex((one) => one.message === message.id) ?? -1;
                  return (
                    <YourMessage
                      key={message.id}
                      row={{
                        id: message.id,
                        at: message.at ?? new Date(0).toISOString(),
                        text: message.text,
                        ...(at < 0
                          ? {}
                          : {
                              queued: true,
                            }),
                      }}
                    />
                  );
                }
                return (
                  <li
                    key={message.id}
                    className="armada-helm-thread__row"
                    data-actor="helm"
                    data-from="agent"
                  >
                    <div className="armada-helm-thread__head">
                      <span className="armada-helm-thread__who">Operator</span>
                    </div>
                    <div className="armada-helm-thread__message">
                      <p className="armada-session-said">{message.text}</p>
                    </div>
                    {message.order === undefined ? null : (
                      <OrderProposed
                        order={message.order}
                        settled={message.settled}
                        onSettle={(keep) =>
                          act({
                            kind: "order",
                            message: message.id,
                            keep,
                          })
                        }
                      />
                    )}
                  </li>
                );
              })}
            </ol>
          </>
        )}
      </div>
      <SessionComposer
        key={item?.id ?? "general"}
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
        draftKey={item === undefined ? "operator:box" : `operator:box:${item.id}`}
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
