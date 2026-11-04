// The Operator panel, in Helm's dock over the whole work: the Log, Waiting to start, Permissions with the standing orders, and the
// Job composer at its foot. Each pane wears the Cockpit HUD's frame. Drawn only where the window has an Operator store (`store.tsx`),
// which today is the mock's alone.

import { useLayoutEffect, useRef, useState } from "react";
import type { DragEvent, KeyboardEvent, ReactNode } from "react";
import { Bot, Lock } from "lucide-react";
import {
  Button,
  DropdownMenu,
  Input,
  JobCard,
  SessionComposer,
  Tooltip,
  type ComposerMode,
  type ComposerTag,
} from "@armada/components";
import type { DrawnSketch } from "@armada/screens/src/draft/sessions";
import { askToCloseHelm } from "@armada/shell";

import { Frame } from "../cockpit/deck/ConsoleHud";
import { select } from "../cockpit/deck/store";
import { searchFiles } from "../commands";
import { useNumbered } from "../numbered";
import { hostedCommands } from "../session-runner";
import { stateOf } from "../session-state";
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
  OperatorJob,
  OperatorLineRow,
  OperatorLogRow,
  OperatorPosition,
  OperatorSource,
} from "./model";
import { useOperator } from "./store";
import type { OperatorStore } from "./store";
import "../cockpit/deck/console-bridge.css";
import "./operator.css";

/** The HUD head strip the pane's name stands in, the timeline HUD's height. */
const HEAD = 40;

export function OperatorPanel() {
  const store = useOperator();
  if (store === null) return null;
  return (
    <div className="armada-operator">
      <Log store={store} />
      <Line store={store} />
      <Permissions store={store} />
      <Box store={store} />
    </div>
  );
}

/** A pane in the HUD's frame: glass, a hairline, proud corners and the name in the head strip. */
function HudPane({
  part,
  label,
  acts,
  children,
}: {
  part: string;
  label: string;
  acts?: ReactNode;
  children: ReactNode;
}) {
  const at = useRef<HTMLElement>(null);
  const [size, setSize] = useState<{
    w: number;
    h: number;
  }>();
  useLayoutEffect(() => {
    const pane = at.current;
    if (pane === null) return;
    const measure = () =>
      setSize({
        w: pane.offsetWidth,
        h: pane.offsetHeight,
      });
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(pane);
    return () => watch.disconnect();
  }, []);
  return (
    <section ref={at} className="armada-operator__pane" data-part={part} aria-label={label}>
      {size === undefined ? null : <Frame w={size.w} h={size.h} head={HEAD} />}
      <h2 className="armada-operator__name">{label}</h2>
      {acts === undefined ? null : <span className="armada-operator__acts">{acts}</span>}
      <div className="armada-operator__body">{children}</div>
    </section>
  );
}

/** A Job a row names, as the shared Job card: its state, its title and its age. A press opens the Job, and the dock steps aside for it. */
function NamedJob({ job }: { job: OperatorJob }) {
  const session = useSessions().find((one) => one.id === job.session);
  const { state, said } =
    session === undefined
      ? {
          state: "ended" as const,
          said: "Ended",
        }
      : stateOf(session);
  return (
    <JobCard
      title={session?.title ?? job.title}
      state={state}
      said={said}
      {...(session?.lastTurnAt === undefined
        ? {}
        : {
            at: session.lastTurnAt,
          })}
      onOpen={() => {
        select({
          kind: "session",
          key: `session:${job.session}`,
        });
        askToCloseHelm();
      }}
    />
  );
}

/** What started the item: a Job as its card, a bug report naming who filed it, or you, Fleet or Waiting to start in words. */
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
    <HudPane part="line" label="Waiting to start">
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
              event.dataTransfer.setData("text/plain", row.id);
              setDragging(row.id);
            }}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => drop(event, row.id)}
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
    </HudPane>
  );
}

function LineText({ row }: { row: OperatorLineRow }) {
  return (
    <span className="armada-operator__line-text">
      <span className="armada-operator__title">{row.title}</span>
      <span className="armada-operator__wait">{waitLine(row.waits)}</span>
      {row.waits.kind === "job" ? <NamedJob job={row.waits.job} /> : null}
    </span>
  );
}

function Log({ store }: { store: OperatorStore }) {
  const now = Date.now();
  return (
    <HudPane part="log" label="Log">
      <ol className="armada-operator__log">
        {store.state.log.map((row) => (
          <LogRow key={row.id} row={row} now={now} store={store} />
        ))}
      </ol>
    </HudPane>
  );
}

function LogRow({ row, now, store }: { row: OperatorLogRow; now: number; store: OperatorStore }) {
  const line =
    row.line === undefined ? undefined : store.state.line.find((one) => one.id === row.line);
  return (
    <li className="armada-operator__log-row" aria-label={`${sourceSaid(row.from)}: ${row.item}`}>
      <span className="armada-operator__source">
        <Source from={row.from} />
      </span>
      <span className="armada-operator__line-text">
        <span className="armada-operator__title">{row.item}</span>
        <span className="armada-operator__why">
          {row.changed === undefined ? row.why : `Overruled by you: ${row.changed}`}
        </span>
      </span>
      <span className="armada-operator__did" data-did={row.did}>
        {DID_SAID[row.did]}
      </span>
      <span className="armada-operator__named">
        {(row.names ?? []).map((job) => (
          <NamedJob key={job.session} job={job} />
        ))}
      </span>
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
    <HudPane part="permissions" label="Permissions">
      <ul className="armada-operator__permissions">
        <li className="armada-operator__permission" data-master="">
          <span className="armada-operator__line-text">
            <span className="armada-operator__title">Operator</span>
            <span className="armada-operator__does">Off sends everything to you as today</span>
          </span>
          <DropdownMenu
            triggerLabel={state.on ? "On" : "Off"}
            triggerAriaLabel={`Operator: ${state.on ? "On" : "Off"}`}
            align="end"
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
    </HudPane>
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

/** The conversation, and the Job composer under it: `@` Jobs and files, `#` issues and pull requests, `/` commands, files and sketches. */
function Box({ store }: { store: OperatorStore }) {
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
    <HudPane
      part="box"
      label="Talk to the Operator"
      acts={
        <Button
          size="sm"
          variant="ghost"
          disabled={state.messages.length === 0}
          onClick={() =>
            act({
              kind: "clear",
            })
          }
        >
          Clear
        </Button>
      }
    >
      {state.messages.length === 0 ? null : (
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
      )}
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
    </HudPane>
  );
}
