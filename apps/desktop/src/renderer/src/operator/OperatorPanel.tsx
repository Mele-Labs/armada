// The Operator panel, in the dock over the whole work, on the panes a Job's area uses (`job-panes/`): tabs and splits he adds,
// closes, moves and resizes, kept as a Job's layout is. Its tabs are Talk, the Log, Waiting to start and Permissions; it opens on
// Talk and the Log in one pane. Drawn only where the window has an Operator store (`store.tsx`), which today is the mock's alone.

import { Fragment, createContext, useContext, useState } from "react";
import type { DragEvent, KeyboardEvent, ReactNode } from "react";
import {
  ArrowRightLeft,
  Bot,
  Check,
  Clock,
  CornerUpRight,
  DoorOpen,
  FileSearch,
  GitMerge,
  GripVertical,
  Hand,
  Link,
  LoaderCircle,
  Lock,
  Megaphone,
  MessageSquareText,
  MessagesSquare,
  Pause,
  Play,
  Power,
  PowerOff,
  RotateCcw,
  Ticket,
  Undo2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import {
  Button,
  DropdownMenu,
  Input,
  SessionComposer,
  SessionThread,
  SpaciousTake,
  SplitButton,
  Switch,
  Tooltip,
  type ComposerMode,
  type ComposerTag,
  type DropdownMenuEntry,
  type SessionThreadRow,
  type ThreadCard,
} from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { DrawnSketch } from "@armada/screens/src/draft/sessions";
import { jobIdOf } from "@armada/shell";

import { searchFiles } from "../commands";
import { JobChip } from "../job-chip";
import { usePlanUsage } from "../plan-usage";
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
  POSITION_DOES,
  POSITION_SAID,
  ROLES,
  offerTitle,
  orderSaid,
  outcomeOf,
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
  OperatorMessage,
  OperatorPosition,
  OperatorRaise,
  StandingOrder,
  OperatorTurn,
} from "./model";
import { takeNamed } from "./takes/take";
import { JobOver, openJobOver } from "./overlay";
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
  if (store === null) return null;
  // The Layout control stands in the title bar beside the context meter, as an open Job's does: the Operator's page and a Job are
  // never drawn at once.
  return (
    <Picked.Provider value={picked}>
      <div className="armada-operator">
        <JobPanes
          jobId={OPERATOR}
          family={operatorPanes}
          body={(tab) => <Body tab={tab} store={store} />}
          add={{
            entries: addEntries,
            onSelect: (id, pane) => {
              operatorPanes.rearrange(OPERATOR, (now) => addTab(now, pane.id, id as TabKind));
              focusIn(pane.id);
            },
          }}
        />
        <JobOver state={store.state} />
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

/** Opens a Job from a chip or a row: over the Operator, which stays as it was behind it. */
const openJob = openJobOver;

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

/** What kind of thing a row is, at a glance: the Operator decided it, it came to you, the Operator acted on a Job, or you did. */
type LogKind = "decided" | "to-you" | "acted" | "you";

const KIND_SAID: Readonly<Record<LogKind, string>> = {
  decided: "Decided",
  "to-you": "Came to you",
  acted: "Coordinated",
  you: "You",
};

const KIND_OF_DID: Readonly<Record<OperatorDid, LogKind>> = {
  answered: "decided",
  overruled: "decided",
  raised: "to-you",
  passed_on: "to-you",
  delegated: "acted",
  existing: "acted",
  merged: "acted",
  in_touch: "acted",
  held: "acted",
  started: "acted",
  filed: "acted",
  released: "acted",
  returned: "acted",
  resumed: "acted",
  moved: "acted",
};

/** The glyph on the Log's spine: what was done. */
const GLYPH_OF_DID: Readonly<Record<OperatorDid, LucideIcon>> = {
  answered: Check,
  overruled: Undo2,
  raised: Hand,
  passed_on: Hand,
  delegated: Megaphone,
  existing: CornerUpRight,
  merged: GitMerge,
  in_touch: Link,
  held: Pause,
  started: Play,
  filed: Ticket,
  released: DoorOpen,
  returned: RotateCcw,
  resumed: Play,
  moved: ArrowRightLeft,
};

const kindOf = (row: OperatorLogRow): LogKind => (row.by === "you" ? "you" : KIND_OF_DID[row.did]);

/** What an item was, as the row's sentence names it. */
const NOUN: Readonly<Record<NonNullable<OperatorLogRow["about"]>, string>> = {
  question: "a question",
  stall: "a quiet Job",
  report: "a bug report",
  start: "a start",
  note: "a note",
};

const nounOf = (row: OperatorLogRow): string =>
  row.about !== undefined
    ? NOUN[row.about]
    : row.from.kind === "report"
      ? NOUN.report
      : row.from.kind === "fleet"
        ? "what Fleet saw"
        : NOUN.question;

/** Words and Jobs in order, a space between each, nothing where a part is absent. */
function said(...parts: ReactNode[]): ReactNode {
  return parts
    .filter((one) => one !== undefined && one !== null && one !== false && one !== "")
    .map((one, at) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: the parts of one sentence keep their order
      <Fragment key={at}>
        {at === 0 ? null : " "}
        {one}
      </Fragment>
    ));
}

const chips = (jobs: readonly OperatorJob[]): ReactNode =>
  jobs.length === 0 ? undefined : jobs.map((one) => <NamedJob key={one.session} job={one} />);

/** The row as one sentence: who did what to which Job. */
function sentenceOf(row: OperatorLogRow, job: OperatorJob | undefined): ReactNode {
  const who = <b className="armada-operator__log-who">{row.by === "you" ? "You" : "Operator"}</b>;
  const at = job === undefined ? undefined : <NamedJob job={job} />;
  const others = chips(row.names ?? []);
  const from = at === undefined ? undefined : said("from", at);
  switch (row.did) {
    case "answered":
      return said(who, "answered", at, row.what === undefined ? undefined : `with ${row.what}`);
    case "raised":
      return row.from.kind === "report" && row.from.by === "you"
        ? said(
            <b className="armada-operator__log-who">You</b>,
            "reported a bug, and the Operator passed it back to you",
          )
        : said(who, "passed you", nounOf(row), from);
    case "passed_on":
      return said(nounOf(row).replace(/^a/, "A"), from, "came to you");
    case "existing":
      return row.by === "you"
        ? said(who, "sent a note", others === undefined ? row.what : said("to", others))
        : said(
            who,
            "handed",
            nounOf(row),
            from,
            others === undefined ? row.what : said("to", others),
          );
    case "delegated":
      return said(who, "nudged", others ?? at);
    case "merged": {
      // The Job taken in, then the one it went into.
      const taken = chips((row.names ?? []).filter((one) => one.as === "merged"));
      const into = chips((row.names ?? []).filter((one) => one.as !== "merged"));
      return said(who, "merged", taken ?? at, into === undefined ? row.what : said("into", into));
    }
    case "in_touch":
      return said(
        who,
        "kept",
        at ?? "it",
        "in touch",
        others === undefined ? row.what : said("with", others),
      );
    case "held":
      return said(who, "held", at ?? others ?? "it", row.what);
    case "started":
      return said(who, "started", others ?? at ?? row.what ?? "a Job");
    case "filed":
      return said(who, "filed", nounOf(row), from, "as an issue");
    default:
      return said(who, DID_SAID[row.did].toLowerCase(), at ?? others ?? "it");
  }
}

const startOfDay = (one: Date) =>
  new Date(one.getFullYear(), one.getMonth(), one.getDate()).getTime();

/** Today, Yesterday, or the day's name and date. */
function dayOf(at: string, now: Date): string {
  const day = new Date(at);
  const days = Math.round((startOfDay(now) - startOfDay(day)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  return day.toLocaleDateString([], {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

const timeOf = (at: string) =>
  new Date(at).toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });

type Entry =
  | {
      kind: "row";
      at: string;
      row: OperatorLogRow;
    }
  | {
      kind: "switch";
      at: string;
      on: boolean;
    };

/** The Log: one sentence a row, newest first, grouped by day, with a line where he switched the Operator on or off. */
function Log({ store }: { store: OperatorStore }) {
  const shown = usePickedFilter();
  const now = new Date();
  const entries: Entry[] = [
    ...store.state.log
      .filter((row) => shown(row.repository))
      .map((row) => ({
        kind: "row" as const,
        at: row.at,
        row,
      })),
    ...(store.state.switched ?? []).map((one) => ({
      kind: "switch" as const,
      at: one.at,
      on: one.on,
    })),
  ].sort((one, other) => Date.parse(other.at) - Date.parse(one.at));
  const days: [
    string,
    Entry[],
  ][] = [];
  for (const entry of entries) {
    const day = dayOf(entry.at, now);
    const last = days.at(-1);
    if (last?.[0] === day) last[1].push(entry);
    else
      days.push([
        day,
        [
          entry,
        ],
      ]);
  }
  return (
    <div className="armada-operator__log">
      {days.map(([day, list]) => (
        <section key={day} className="armada-operator__log-day" aria-label={day}>
          <h3 className="armada-operator__log-heading">{day}</h3>
          <ol className="armada-operator__log-list">
            {list.map((entry) =>
              entry.kind === "switch" ? (
                <li
                  key={`switch ${entry.at}`}
                  className="armada-operator__log-switch"
                  data-on={entry.on || undefined}
                >
                  <span className="armada-operator__log-time">{timeOf(entry.at)}</span>
                  <span className="armada-operator__log-node">
                    {entry.on ? (
                      <Power size={12} strokeWidth={2.25} aria-hidden />
                    ) : (
                      <PowerOff size={12} strokeWidth={2.25} aria-hidden />
                    )}
                  </span>
                  <span className="armada-operator__log-switched">
                    Operator switched {entry.on ? "on" : "off"}
                  </span>
                </li>
              ) : (
                <LogRow key={entry.row.id} row={entry.row} />
              ),
            )}
          </ol>
        </section>
      ))}
    </div>
  );
}

/** One row: its time, the Operator's mark on its own rows, the sentence, the words it was about, why, and Open Job. */
function LogRow({ row }: { row: OperatorLogRow }) {
  const job =
    row.from.kind === "job"
      ? row.from.job
      : row.from.kind === "report" && row.from.by !== "you"
        ? row.from.by
        : undefined;
  const quote = row.item === job?.title ? undefined : row.item;
  const open = job ?? row.names?.[0];
  const kind = kindOf(row);
  const Glyph =
    row.by === "you" && row.did === "existing" ? MessageSquareText : GLYPH_OF_DID[row.did];
  return (
    <li className="armada-operator__log-row" data-kind={kind}>
      <span className="armada-operator__log-time">{timeOf(row.at)}</span>
      <span className="armada-operator__log-node">
        <Glyph size={12} strokeWidth={2.25} aria-hidden />
      </span>
      <div className="armada-operator__log-card">
        <div className="armada-operator__log-head">
          <span className="armada-operator__log-kind">
            {kind === "you" ? null : <Bot size={12} strokeWidth={2.25} aria-hidden />}
            {KIND_SAID[kind]}
          </span>
          {open === undefined ? null : (
            <Button
              size="sm"
              variant="secondary"
              aria-label={`Open Job: ${open.title}`}
              onClick={() => openJob(open.session)}
            >
              Open Job
            </Button>
          )}
        </div>
        <span className="armada-operator__log-said">{sentenceOf(row, job)}</span>
        {quote === undefined ? null : <span className="armada-operator__log-quote">“{quote}”</span>}
        {row.why === "" ? null : <span className="armada-operator__log-why">{row.why}</span>}
        {row.changed === undefined ? null : (
          <span className="armada-operator__log-why">You changed it to {row.changed}</span>
        )}
      </div>
    </li>
  );
}

const POSITIONS: readonly OperatorPosition[] = [
  "read_only",
  "suggest",
  "decide",
];

function Permissions({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  // Where the week's plan stands, from the meter the title bar reads.
  const week = usePlanUsage()?.find((one) => one.kind === "seven_day")?.percentUsed;
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
                    Read-only in {repository} by standing order
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
                  description: POSITION_DOES[one],
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
            <span className="armada-operator__title">Pause past</span>
            <span className="armada-operator__does">
              {week === undefined
                ? "Of the week's plan. Past it, everything comes to you as today"
                : `Of the week's plan, now at ${Math.round(week)}%. Past it, everything comes to you as today`}
            </span>
          </span>
          <span className="armada-operator__ceiling">
            <Input
              type="number"
              min={0}
              max={100}
              aria-label="Pause past this share of the week's plan"
              unit="%"
              value={state.pause}
              onChange={(event) =>
                act({
                  kind: "pause",
                  share: Number(event.target.value),
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

/** An option's label without the Job's own "(Recommended)", which Go with already says. */
const unmarked = (way: string) => way.replace(/\s*\(recommended\)\s*$/i, "");

/** The item being discussed, pinned above the conversation until he stops discussing it, with every act on it in reach. */
function ItemCard({ store, raise }: { store: OperatorStore; raise: OperatorRaise }) {
  const { act } = store;
  const open = raise.settled === undefined;
  const pressed = (way: string) =>
    act({
      kind: "raise",
      raise: raise.id,
      way,
    });
  const others = raise.ways.filter((way) => way !== raise.recommends && way !== "Open Job");
  const job = raise.job;
  return (
    <div className="armada-operator__card" role="group" aria-label={`Discussing: ${raise.item}`}>
      <span className="armada-operator__card-band">
        <span>Discussing</span>
        {raise.tag === undefined ? null : <span>{raise.tag}</span>}
        <span className="armada-operator__repo">{raise.repository}</span>
      </span>
      <span className="armada-operator__card-body">
        <span className="armada-operator__row-title">{raise.item}</span>
        <span className="armada-operator__raise-read">{raise.read}</span>
        {open ? null : <span className="armada-operator__does">{raise.settled}</span>}
        <span className="armada-operator__card-acts">
          {open && raise.recommends !== undefined ? (
            <Button size="sm" variant="primary" onClick={() => pressed(raise.recommends!)}>
              Go with {unmarked(raise.recommends)}
            </Button>
          ) : null}
          {open
            ? others.map((way) => (
                <Button key={way} size="sm" variant="secondary" onClick={() => pressed(way)}>
                  {way}
                </Button>
              ))
            : null}
          {job === undefined ? null : (
            <>
              <Button size="sm" variant="secondary" onClick={() => openJob(job.session)}>
                Open Job
              </Button>
              <Tooltip label="Starts a Job that reads the code and answers this. It changes nothing">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() =>
                    act({
                      kind: "dispatch",
                      raise: raise.id,
                      what: "investigate",
                    })
                  }
                >
                  Dispatch investigation
                </Button>
              </Tooltip>
              <Tooltip label="Starts a Job that reviews this Job's work. It changes nothing">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() =>
                    act({
                      kind: "dispatch",
                      raise: raise.id,
                      what: "review",
                    })
                  }
                >
                  Dispatch review
                </Button>
              </Tooltip>
            </>
          )}
          <Button
            size="sm"
            variant="ghost"
            data-stop=""
            onClick={() =>
              act({
                kind: "not-now",
              })
            }
          >
            Stop discussing
          </Button>
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

/** A Job the Operator offered to read the code: its brief, and Dispatch into a repository, or Leave it. */
function OfferedJob({
  offer,
  onSettle,
}: {
  offer: NonNullable<OperatorMessage["offer"]>;
  onSettle: (repository: string | null) => void;
}) {
  const title = offerTitle(offer.brief);
  const only = offer.in.length === 1 ? offer.in[0] : undefined;
  return (
    <span className="armada-operator__offer" role="group" aria-label={`Job offered: ${title}`}>
      <span className="armada-operator__offer-kind">
        <FileSearch size={12} strokeWidth={2} aria-hidden />
        Job offered
      </span>
      <span className="armada-operator__offer-brief">{offer.brief}</span>
      {offer.left === true ? (
        <span className="armada-operator__does">Left</span>
      ) : (
        <span className="armada-operator__offer-acts">
          {offer.in.length === 0 ? (
            <span className="armada-operator__does">
              No repository is served here to start it in
            </span>
          ) : only !== undefined ? (
            <Button
              size="sm"
              variant="secondary"
              aria-label={`Dispatch in ${only}: ${title}`}
              onClick={() => onSettle(only)}
            >
              Dispatch
            </Button>
          ) : (
            <DropdownMenu
              triggerLabel="Dispatch"
              triggerAriaLabel={`Dispatch: ${title}`}
              align="start"
              portal
              entries={offer.in.map((one) => ({
                kind: "item" as const,
                id: one,
                label: `In ${one}`,
              }))}
              onSelect={(one) => onSettle(one)}
            />
          )}
          <Button size="sm" variant="secondary" onClick={() => onSettle(null)}>
            Leave it
          </Button>
        </span>
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
  // A turn about a Job is named by the Job, with what it asks after.
  const named = (turn: OperatorTurn) =>
    turn.job === undefined || turn.job.title === turn.text
      ? `${turn.text.replace(/[.?!]$/, "")}.`
      : `${turn.job.title}: ${turn.text}`;
  const said = [
    open[0] === undefined ? null : `I'm on ${named(open[0])}`,
    open[1] === undefined ? null : `Next is ${named(open[1])}`,
    first === undefined ? null : `First for you: ${first.item}, under Needs you.`,
  ].filter((one) => one !== null);
  return said.length === 0 ? null : said.join(" ");
}

/** When a message the mock wrote without a time was said: in the order it came, from when the window opened. */
const OPENED = Date.now();
const atOf = (message: OperatorMessage, index: number) =>
  message.at ?? new Date(OPENED + index * 1000).toISOString();

/** Discussion's words as a Job thread's rows, so they read as a Job's thread does. Nothing said yet, the Operator opens. */
function threadRows(store: OperatorStore): SessionThreadRow[] {
  const { messages, turns } = store.state;
  if (messages.length === 0) {
    const said = openingOf(store);
    return said === null
      ? []
      : [
          {
            id: "opening",
            at: new Date(OPENED).toISOString(),
            kind: "message",
            from: "agent",
            text: said,
          },
        ];
  }
  return messages.flatMap((message, index): SessionThreadRow[] =>
    message.card !== undefined || message.opens !== undefined || message.text === ""
      ? []
      : [
          {
            id: message.id,
            at: atOf(message, index),
            kind: "message",
            from: message.from === "you" ? "you" : "agent",
            text: message.text,
            ...(message.from === "you" && turns?.some((one) => one.message === message.id) === true
              ? {
                  queued: true as const,
                }
              : {}),
          },
        ],
  );
}

/** What Discussion draws between its words: where an item came up or was settled, a Job it started, an order it proposed. */
function threadCards(store: OperatorStore): ThreadCard[] {
  const { act } = store;
  return store.state.messages.flatMap((message, index): ThreadCard[] => {
    const at = atOf(message, index);
    const cards: ThreadCard[] = [];
    if (message.card !== undefined)
      cards.push({
        key: message.id,
        at,
        node: <CardMarker store={store} id={message.card} ends={message.ends === true} />,
      });
    if (message.opens !== undefined) {
      const opens = message.opens;
      cards.push({
        key: `${message.id}:opens`,
        at,
        node: (
          <span className="armada-operator__marker armada-operator__started">
            <span className="armada-operator__marker-item">{message.text}</span>
            <Button
              size="sm"
              variant="secondary"
              aria-label={`Open Job: ${opens.title}`}
              onClick={() => openJob(opens.session)}
            >
              Open Job
            </Button>
          </span>
        ),
      });
    }
    if (message.offer !== undefined && message.opens === undefined)
      cards.push({
        key: `${message.id}:offer`,
        at,
        node: (
          <OfferedJob
            offer={message.offer}
            onSettle={(repository) =>
              act({
                kind: "offer",
                message: message.id,
                repository,
              })
            }
          />
        ),
      });
    if (message.order !== undefined) {
      const order = message.order;
      cards.push({
        key: `${message.id}:order`,
        at,
        node: (
          <OrderProposed
            order={order}
            settled={message.settled}
            onSettle={(keep) =>
              act({
                kind: "order",
                message: message.id,
                keep,
              })
            }
          />
        ),
      });
    }
    return cards;
  });
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
  const off = !state.on;
  const rows = threadRows(store);
  const first = rows[0]?.at;
  const cards = threadCards(store);
  const leading = cards.filter((one) => first === undefined || (one.at ?? "") < first);
  const after = cards.filter((one) => !leading.includes(one));
  return (
    <SpaciousTake.Provider value>
      <div className="armada-operator__talk-pane armada-session-frame" data-off={off || undefined}>
        {off ? (
          <div className="armada-operator__off">
            <Switch
              size="sm"
              checked={false}
              description="On, it answers here. What it may do to your Jobs is set under Permissions"
              onChange={() =>
                act({
                  kind: "on",
                  on: true,
                })
              }
            >
              Operator
            </Switch>
          </div>
        ) : null}
        {item === undefined ? null : <ItemCard store={store} raise={item} />}
        <div className="armada-operator__conversation">
          {messages.length === 0 ? null : (
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
          )}
          <SessionThread
            sessionId="operator"
            agentName="Operator"
            rows={rows}
            // A card said before any words leads the thread: the thread places the rest after the words before them.
            {...(leading.length === 0
              ? {}
              : {
                  lead: leading.map((one) => <Fragment key={one.key}>{one.node}</Fragment>),
                })}
            inline={after}
            onAnswer={() => undefined}
            onOpenSession={openJob}
          />
        </div>
        <SessionComposer
          key={item?.id ?? "general"}
          label="Message to the Operator"
          disabled={off}
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
    </SpaciousTake.Provider>
  );
}
