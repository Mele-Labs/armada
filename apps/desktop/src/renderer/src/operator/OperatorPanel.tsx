// The Operator panel, in Helm's dock: the Line, the Log, Permissions with the standing orders, and the box at its foot. Drawn only
// where the window has an Operator store (`store.tsx`), which today is the mock's alone.

import { useState } from "react";
import type { DragEvent, KeyboardEvent } from "react";
import { Activity, Bot, CirclePause, Clock, Lock } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  DropdownMenu,
  Input,
  Textarea,
  Tooltip,
} from "@armada/components";

import { DID_SAID, POSITION_SAID, ROLES, ageOf, orderSaid, waitSaid } from "./model";
import type { OperatorLineRow, OperatorLogRow, OperatorPosition, OperatorWait } from "./model";
import { useOperator } from "./store";
import type { OperatorStore } from "./store";
import "./operator.css";

const WAIT_GLYPH: Readonly<Record<OperatorWait["kind"], LucideIcon>> = {
  job: Clock,
  meter: Activity,
  you: CirclePause,
};

export function OperatorPanel() {
  const store = useOperator();
  if (store === null) return null;
  return (
    <div className="armada-operator" data-take={store.state.take}>
      <Line store={store} />
      <Log store={store} />
      <Permissions store={store} />
      <Box store={store} />
    </div>
  );
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
    <Card className="armada-operator__pane" data-part="line" role="region" aria-label="Line">
      <CardHeader>
        <CardTitle>Line</CardTitle>
      </CardHeader>
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
    </Card>
  );
}

function LineText({ row }: { row: OperatorLineRow }) {
  const Glyph = WAIT_GLYPH[row.waits.kind];
  return (
    <span className="armada-operator__line-text">
      <span className="armada-operator__title">{row.title}</span>
      <span className="armada-operator__wait">
        <Glyph size={12} strokeWidth={2} aria-hidden />
        {waitSaid(row.waits)}
      </span>
    </span>
  );
}

function Log({ store }: { store: OperatorStore }) {
  const now = Date.now();
  return (
    <Card className="armada-operator__pane" data-part="log" role="region" aria-label="Log">
      <CardHeader>
        <CardTitle>Log</CardTitle>
      </CardHeader>
      <ol className="armada-operator__log">
        {store.state.log.map((row) => (
          <LogRow key={row.id} row={row} now={now} store={store} />
        ))}
      </ol>
    </Card>
  );
}

function LogRow({ row, now, store }: { row: OperatorLogRow; now: number; store: OperatorStore }) {
  const line =
    row.line === undefined ? undefined : store.state.line.find((one) => one.id === row.line);
  return (
    <li className="armada-operator__log-row" aria-label={`${row.from}: ${row.item}`}>
      <span className="armada-operator__title">{row.from}</span>
      <span className="armada-operator__item">{row.item}</span>
      <span className="armada-operator__did" data-did={row.did}>
        {DID_SAID[row.did]}
      </span>
      <span className="armada-operator__why">
        {row.changed === undefined ? row.why : `Changed by you: ${row.changed}`}
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
            triggerLabel="Change"
            triggerAriaLabel={`Change: ${row.item}`}
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
    <Card
      className="armada-operator__pane"
      data-part="permissions"
      role="region"
      aria-label="Permissions"
    >
      <CardHeader>
        <CardTitle>Permissions</CardTitle>
      </CardHeader>
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
      <CardHeader>
        <CardTitle>Standing orders</CardTitle>
      </CardHeader>
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
    </Card>
  );
}

function Box({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const [words, setWords] = useState("");
  const send = () => {
    if (words.trim() === "") return;
    act({
      kind: "say",
      text: words,
    });
    setWords("");
  };
  return (
    <section className="armada-operator__box" aria-label="Talk to the Operator">
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
      <div className="armada-operator__compose">
        <Textarea
          aria-label="Message to the Operator"
          placeholder="Tell the Operator something, or ask it why"
          keep="operator:box"
          rows={1}
          grow
          value={words}
          onChange={(event) => setWords(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
            event.preventDefault();
            send();
          }}
        />
        <Button size="sm" variant="primary" disabled={words.trim() === ""} onClick={send}>
          Send
        </Button>
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
      </div>
    </section>
  );
}
