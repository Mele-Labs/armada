// The board take: the Operator's work as lanes in the order it flows: Now, the card it is on, then Up next, Needs you and Done. A card keeps its place on the board as it moves, and slides from where it was to where it went. Discussion stays the panel's own.

import { Children, useLayoutEffect, useRef, useState } from "react";
import type { DragEvent, KeyboardEvent, ReactNode } from "react";
import { Bot, Check, Clock, LoaderCircle, MessagesSquare } from "lucide-react";
import { Button, SplitButton, Tooltip, timeSaid, useTimeFormat } from "@armada/components";
import { askToCloseOperator } from "@armada/shell";

import { select } from "../../cockpit/deck/store";
import { JobChip } from "../../job-chip";
import { operatorPanes } from "../../job-panes/store";
import { activate, addTab, holderOf, panesOf } from "../../job-panes/tree";
import { outcomeOf, turnLine, turnTag } from "../model";
import type { OperatorRaise, OperatorTurn } from "../model";
import type { OperatorStore } from "../store";
import type { OperatorTake } from "./take";
import "./board.css";

const OPERATOR = "operator";

export const openJob = (id: string) => {
  select({
    kind: "session",
    key: `session:${id}`,
  });
  askToCloseOperator();
};

/** Discussion in front, so an item brought up there is seen. */
function showTalk() {
  operatorPanes.rearrange(OPERATOR, (now) => {
    const pane = holderOf(now, "op-talk");
    const tab = pane?.tabs.find((one) => one.kind === "op-talk");
    if (tab !== undefined) return activate(now, tab.id);
    const first = panesOf(now.root)[0];
    return first === undefined ? now : addTab(now, first.id, "op-talk");
  });
}

/**
 * Each card slides from where it was drawn last to where it is now, across lanes too, since it is found by its id. Only a
 * card that changed lane or place in its lane slides: a scroll, a resize or a refresh that moved nothing redraws still.
 * Places are read against the board, so a scrolled pane does not read as a move.
 */
function useSlide(): (node: HTMLElement | null) => void {
  const root = useRef<HTMLElement | null>(null);
  const was = useRef(
    new Map<
      string,
      {
        x: number;
        y: number;
        place: string;
      }
    >(),
  );
  useLayoutEffect(() => {
    const board = root.current;
    if (board === null) return;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const origin = board.getBoundingClientRect();
    const now = new Map<
      string,
      {
        x: number;
        y: number;
        place: string;
      }
    >();
    for (const card of board.querySelectorAll<HTMLElement>("[data-card]")) {
      const id = card.dataset.card!;
      const rect = card.getBoundingClientRect();
      const lane = card.closest<HTMLElement>("[data-lane]")?.dataset.lane ?? "";
      const at = {
        x: rect.left - origin.left,
        y: rect.top - origin.top,
        place: `${lane}:${[
          ...(card.parentElement?.children ?? []),
        ].indexOf(card)}`,
      };
      now.set(id, at);
      const from = was.current.get(id);
      if (from === undefined || still || from.place === at.place) continue;
      const dx = from.x - at.x;
      const dy = from.y - at.y;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) continue;
      card.animate(
        [
          {
            transform: `translate(${dx}px, ${dy}px)`,
          },
          {
            transform: "none",
          },
        ],
        {
          duration: 360,
          easing: "cubic-bezier(0.2, 0.8, 0.2, 1)",
        },
      );
    }
    was.current = now;
  });
  return (node) => {
    root.current = node;
  };
}

type Dragged = {
  kind: "turn" | "raise";
  id: string;
};

const readDrag = (event: DragEvent): Dragged | null => {
  const [kind, id] = event.dataTransfer.getData("text/plain").split(":");
  return (kind === "turn" || kind === "raise") && id !== undefined && id !== ""
    ? {
        kind,
        id,
      }
    : null;
};

function Card({
  id,
  tag,
  repository,
  title,
  line,
  outcome,
  glyph,
  said,
  state,
  act,
  drag,
  dropOn,
  place,
  onLift,
  onKey,
  who,
  when,
  detail,
  job,
}: {
  id: string;
  tag: string;
  repository?: string;
  title: string;
  line?: string;
  /** What came of it, as a callout in place of the line. */
  outcome?: string;
  glyph: ReactNode;
  said: string;
  state?: "on-it" | "done" | "discussing";
  act?: ReactNode;
  drag?: Dragged;
  dropOn?: (event: DragEvent<HTMLLIElement>) => void;
  /** Where a card dropped here lands, beside this one. */
  place?: () => "above" | "below";
  onLift?: () => void;
  onKey?: (event: KeyboardEvent<HTMLLIElement>) => void;
  /** Who settled it, read beside when. */
  who?: string;
  /** ISO time it was settled. */
  when?: string;
  /** What a press on the card opens under it. Absent, the card does not open. */
  detail?: ReactNode;
  /** The Job it is about: its title is drawn as the Job, with its state's mark, and a press on it opens the Job. */
  job?: {
    session: string;
    title: string;
  };
}) {
  const [over, setOver] = useState<"above" | "below" | null>(null);
  const [opened, setOpened] = useState(false);
  const format = useTimeFormat();
  // A press on one of the card's own buttons is that button's, not the card's.
  const toggle = (target: EventTarget) => {
    if (detail === undefined || (target as HTMLElement).closest("button, a") !== null) return;
    setOpened((was) => !was);
  };
  return (
    <li
      className="armada-board__card"
      data-card={id}
      data-state={state}
      data-over={over ?? undefined}
      aria-label={title}
      tabIndex={0}
      draggable={drag !== undefined}
      {...(drag === undefined
        ? {}
        : {
            "aria-description": "Drag, or ⌥↑ and ⌥↓, to change what the Operator takes next",
          })}
      onDragStart={(event) => {
        if (drag === undefined) return;
        // The card's own drag, not a tab's: the panes' drop targets keep out of it.
        event.stopPropagation();
        event.dataTransfer.setData("text/plain", `${drag.kind}:${drag.id}`);
        onLift?.();
      }}
      onDragOver={(event) => {
        if (dropOn === undefined) return;
        event.preventDefault();
        event.stopPropagation();
        setOver(place?.() ?? "above");
      }}
      onDragLeave={() => setOver(null)}
      onDrop={(event) => {
        setOver(null);
        if (dropOn === undefined) return;
        event.preventDefault();
        event.stopPropagation();
        dropOn(event);
      }}
      onKeyDown={(event) => {
        if (
          detail !== undefined &&
          (event.key === "Enter" || event.key === " ") &&
          event.target === event.currentTarget
        ) {
          event.preventDefault();
          setOpened((was) => !was);
          return;
        }
        onKey?.(event);
      }}
      onClick={(event) => toggle(event.target)}
      {...(detail === undefined
        ? {}
        : {
            "aria-expanded": opened,
            "data-opens": "",
          })}
    >
      <span className="armada-board__card-head">
        <Tooltip label={said}>
          <span role="img" aria-label={said} className="armada-board__glyph">
            {glyph}
          </span>
        </Tooltip>
        <span className="armada-board__tag">{tag}</span>
      </span>
      {job === undefined ? (
        <span className="armada-board__title">{title}</span>
      ) : (
        <JobChip
          className="armada-board__title armada-board__job"
          job={{
            id: job.session,
            title: job.title,
          }}
          onOpen={openJob}
        />
      )}
      {line === undefined ? null : <span className="armada-board__line">{line}</span>}
      {outcome === undefined ? null : <Outcome text={outcome} />}
      {who === undefined && when === undefined ? null : (
        <span className="armada-board__who">
          {[
            who,
            when === undefined ? undefined : timeSaid(when, format),
          ]
            .filter((one) => one !== undefined)
            .join(" · ")}
        </span>
      )}
      {opened && detail !== undefined ? (
        <span className="armada-board__detail">{detail}</span>
      ) : null}
      {act === undefined && (repository === undefined || repository === "") ? null : (
        <span className="armada-board__foot">
          {act === undefined ? null : <span className="armada-board__act">{act}</span>}
          {repository === undefined || repository === "" ? null : (
            <Tooltip label="Repository">
              <span className="armada-board__repo">{repository}</span>
            </Tooltip>
          )}
        </span>
      )}
    </li>
  );
}

/** What a card about a Job asks, under the Job's name. Absent where its words are the Job's name, as a Job to start is. */
const asks = (turn: OperatorTurn): string | undefined =>
  turn.job === undefined || turn.job.title === turn.text ? undefined : turn.text;

/** What a settled card opens to: what was asked, why it went the way it did, and the Job it was about. Nothing kept, it does not open. */
function detailOf({ asked, why, job }: { asked?: string; why?: string; job?: string }) {
  const rows = [
    [
      "Asked",
      asked,
    ],
    [
      "Why",
      why,
    ],
    [
      "Job",
      job,
    ],
  ].filter(
    (
      row,
    ): row is [
      string,
      string,
    ] => row[1] !== undefined && row[1] !== "",
  );
  if (rows.length === 0) return {};
  return {
    detail: (
      <dl className="armada-board__detail-rows">
        {rows.map(([name, said]) => (
          <span key={name} className="armada-board__detail-row">
            <dt>{name}</dt>
            <dd>{said}</dd>
          </span>
        ))}
      </dl>
    ),
  };
}

/** What came of a card: its kind in caps, and what, in the hue of how it ended. */
function Outcome({ text }: { text: string }) {
  const { label, what, letGo } = outcomeOf(text);
  return (
    <span className="armada-board__outcome" data-let-go={letGo || undefined}>
      <span className="armada-board__outcome-label">{label}</span>
      {what === undefined ? null : <span className="armada-board__outcome-what">{what}</span>}
    </span>
  );
}

/** An item's acts as one split button: the Operator's pick on the face, the rest behind the caret. */
function RaiseActs({
  store,
  raise,
  busy,
}: {
  store: OperatorStore;
  raise: OperatorRaise;
  busy: boolean;
}) {
  const { state, act } = store;
  const [face, ...rest] = raise.ways;
  if (face === undefined) return null;
  const pressed = (way: string) =>
    act({
      kind: "raise",
      raise: raise.id,
      way,
    });
  const discussing = state.open === raise.id;
  const items = [
    ...(discussing
      ? []
      : [
          {
            label: busy ? "Discuss when ready" : "Discuss now",
            note: busy
              ? "Goes into Up next, and comes up in Discussion when the Operator gets to it"
              : "Brings it up in Discussion now",
            onSelect: () => {
              act({
                kind: "discuss",
                raise: raise.id,
                now: !busy,
              });
              if (!busy) showTalk();
            },
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

const SPIN = <LoaderCircle size={12} strokeWidth={2} aria-hidden />;

function Lane({
  name,
  hue,
  empty,
  children,
  onDrop,
}: {
  name: string;
  hue: "now" | "work" | "you" | "done";
  /** What an empty lane says, in a quiet slot a card's size, so the lane reads as empty and not as missing. */
  empty: string;
  children: ReactNode;
  onDrop?: (event: DragEvent<HTMLElement>) => void;
}) {
  return (
    <section
      className="armada-board__lane"
      data-hue={hue}
      data-lane={hue}
      aria-label={name}
      onDragOver={
        onDrop === undefined
          ? undefined
          : (event) => {
              event.preventDefault();
              event.stopPropagation();
            }
      }
      onDrop={
        onDrop === undefined
          ? undefined
          : (event) => {
              event.preventDefault();
              event.stopPropagation();
              onDrop(event);
            }
      }
    >
      <h3 className="armada-board__band">{name}</h3>
      <ol className="armada-board__cards">
        {Children.toArray(children).length === 0 ? (
          <li className="armada-board__empty">{empty}</li>
        ) : (
          children
        )}
      </ol>
    </section>
  );
}

function Queue({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const slide = useSlide();
  const turns = state.turns ?? [];
  const open = turns.filter((one) => one.done === undefined);
  const [onIt, ...next] = open;
  const busy = open.length > 0 && state.discussesAtOnce !== true;
  const raises = state.raises ?? [];
  const raiseOf = (turn: OperatorTurn) => raises.find((one) => one.id === turn.raise);
  const waiting = raises.filter((one) => one.settled === undefined && one.toDiscuss !== true);
  // An act he pressed that the Operator does in its turn stays in Up next until done.
  const pending = new Set(
    open.flatMap((one) =>
      one.raise === undefined
        ? []
        : [
            one.raise,
          ],
    ),
  );
  const settled = raises.filter((one) => one.settled !== undefined && !pending.has(one.id));
  const finished = turns.filter((one) => one.done !== undefined);
  const ids = next.map((one) => one.id);
  // The card being dragged, read while it is over another: a drop takes that card's place, before it moving up and after it moving down.
  const lifted = useRef<string | null>(null);
  const placeBy = (target: string) => (): "above" | "below" => {
    const from = lifted.current === null ? -1 : ids.indexOf(lifted.current);
    return from !== -1 && from < ids.indexOf(target) ? "below" : "above";
  };

  const moveTurn = (turn: string, before: string | null) =>
    act({
      kind: "move-turn",
      turn,
      before,
    });
  // A card from Needs you dropped in Up next is talked over when the Operator gets to it.
  const queueTalk = (raise: string) =>
    act({
      kind: "discuss",
      raise,
      now: false,
    });
  // Dropped on the card it is on, a card is taken next.
  const dropNext = (event: DragEvent<HTMLLIElement>) => {
    const dragged = readDrag(event);
    if (dragged?.kind === "raise") queueTalk(dragged.id);
    else if (dragged?.kind === "turn") moveTurn(dragged.id, ids[0] ?? null);
  };
  const dropBefore = (target: string) => (event: DragEvent<HTMLLIElement>) => {
    const dragged = readDrag(event);
    if (dragged === null) return;
    if (dragged.kind === "raise") return queueTalk(dragged.id);
    const from = ids.indexOf(dragged.id);
    const at = ids.indexOf(target);
    moveTurn(dragged.id, from !== -1 && from < at ? (ids[at + 1] ?? null) : target);
  };
  const keyed = (turn: string) => (event: KeyboardEvent<HTMLLIElement>) => {
    if (!event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    const at = ids.indexOf(turn);
    if (event.key === "ArrowUp" && at > 0) moveTurn(turn, ids[at - 1]!);
    if (event.key === "ArrowDown" && at < ids.length - 1) moveTurn(turn, ids[at + 2] ?? null);
  };

  /** A row of Up next: a raise it will bring up is drawn as that raise, so the card is the same one wherever it goes. */
  const upNext = (turn: OperatorTurn, live: boolean) => {
    const raise = raiseOf(turn);
    const line = live ? undefined : turnLine(open, open.indexOf(turn));
    const act = live ? undefined : turn.discuss === true && turn.raise !== undefined ? (
      <Button
        size="sm"
        variant="secondary"
        aria-label={`Discuss now: ${turn.text}`}
        onClick={() => {
          store.act({
            kind: "discuss",
            raise: turn.raise!,
            now: true,
          });
          showTalk();
        }}
      >
        Discuss now
      </Button>
    ) : turn.start !== undefined ? (
      <Button
        size="sm"
        variant="secondary"
        aria-label={`Start without waiting: ${turn.text}`}
        onClick={() =>
          store.act({
            kind: "start-turn",
            turn: turn.id,
          })
        }
      >
        Start without waiting
      </Button>
    ) : undefined;
    const repository = raise?.repository ?? turn.start?.repository ?? turn.repository;
    return (
      <Card
        key={raise?.id ?? turn.id}
        id={raise?.id ?? turn.id}
        tag={turnTag(turn)}
        {...(repository === undefined
          ? {}
          : {
              repository,
            })}
        title={turn.job?.title ?? turn.text}
        {...(turn.job === undefined
          ? {}
          : {
              job: turn.job,
            })}
        {...(asks(turn) === undefined && (line === undefined || line === "")
          ? {}
          : {
              line:
                turn.discuss === true
                  ? "Comes up in Discussion when the Operator gets here"
                  : turn.result !== undefined
                    ? "You pressed it. The Operator does it in its turn"
                    : (asks(turn) ?? line!),
            })}
        glyph={live ? SPIN : <Clock size={12} strokeWidth={2} aria-hidden />}
        said={live ? "The Operator is on this" : "Waits its turn"}
        {...(live
          ? {
              state: "on-it" as const,
              dropOn: dropNext,
              place: () => "below" as const,
            }
          : {
              drag: {
                kind: "turn" as const,
                id: turn.id,
              },
              dropOn: dropBefore(turn.id),
              place: placeBy(turn.id),
              onLift: () => {
                lifted.current = turn.id;
              },
              onKey: keyed(turn.id),
            })}
        {...(act === undefined
          ? {}
          : {
              act,
            })}
      />
    );
  };

  return (
    <div className="armada-board__room">
      <div className="armada-board" ref={slide}>
        <Lane name="Now" hue="now" empty="Nothing in progress">
          {onIt === undefined ? null : upNext(onIt, true)}
        </Lane>
        <Lane
          name="Up next"
          hue="work"
          empty="Nothing waiting"
          onDrop={(event) => {
            const dragged = readDrag(event);
            if (dragged?.kind === "raise") queueTalk(dragged.id);
            else if (dragged?.kind === "turn") moveTurn(dragged.id, null);
          }}
        >
          {next.map((one) => upNext(one, false))}
        </Lane>
        <Lane name="Needs you" hue="you" empty="Nothing needs you">
          {waiting.map((one) => {
            const discussing = state.open === one.id;
            return (
              <Card
                key={one.id}
                id={one.id}
                tag={one.tag ?? "Item"}
                repository={one.repository}
                title={one.item}
                {...(one.job === undefined
                  ? {}
                  : {
                      job: one.job,
                    })}
                line={discussing ? "In Discussion now" : one.read}
                glyph={
                  discussing ? (
                    <MessagesSquare size={12} strokeWidth={2} aria-hidden />
                  ) : (
                    <Bot size={12} strokeWidth={2} aria-hidden />
                  )
                }
                said={discussing ? "Being discussed" : "The Operator's pick is on the button"}
                {...(discussing
                  ? {
                      state: "discussing" as const,
                    }
                  : {})}
                drag={{
                  kind: "raise",
                  id: one.id,
                }}
                onLift={() => {
                  lifted.current = null;
                }}
                act={<RaiseActs store={store} raise={one} busy={busy} />}
              />
            );
          })}
        </Lane>
        <Lane name="Done" hue="done" empty="Nothing settled yet">
          {settled.map((one) => (
            <Card
              key={one.id}
              id={one.id}
              tag={one.tag ?? "Item"}
              repository={one.repository}
              title={one.item}
              {...(one.job === undefined
                ? {}
                : {
                    job: one.job,
                  })}
              outcome={one.settled!}
              glyph={<Check size={12} strokeWidth={2} aria-hidden />}
              said="Settled"
              state="done"
              {...(one.settledBy === undefined
                ? {}
                : {
                    who: one.settledBy,
                  })}
              {...(one.settledAt === undefined
                ? {}
                : {
                    when: one.settledAt,
                  })}
              {...detailOf({
                asked: one.read,
              })}
              {...(one.job === undefined
                ? {}
                : {
                    act: (
                      <Button
                        size="sm"
                        variant="secondary"
                        aria-label={`Open Job: ${one.job.title}`}
                        onClick={() => openJob(one.job!.session)}
                      >
                        Open Job
                      </Button>
                    ),
                  })}
            />
          ))}
          {finished.map((one) => (
            <Card
              key={one.id}
              id={one.id}
              tag={turnTag(one)}
              title={one.job?.title ?? one.text}
              {...(one.job === undefined
                ? {}
                : {
                    job: one.job,
                  })}
              {...(asks(one) === undefined
                ? {}
                : {
                    line: asks(one),
                  })}
              outcome={one.done!}
              glyph={<Check size={12} strokeWidth={2} aria-hidden />}
              said="Settled"
              state="done"
              {...(one.by === undefined
                ? {}
                : {
                    who: one.by,
                  })}
              {...(one.at === undefined
                ? {}
                : {
                    when: one.at,
                  })}
              {...(one.job === undefined
                ? {}
                : {
                    act: (
                      <Button
                        size="sm"
                        variant="secondary"
                        aria-label={`Open Job: ${one.job.title}`}
                        onClick={() => openJob(one.job!.session)}
                      >
                        Open Job
                      </Button>
                    ),
                  })}
              {...detailOf({
                ...(one.because === undefined
                  ? {}
                  : {
                      why: one.because,
                    }),
                ...(one.job === undefined
                  ? {}
                  : {
                      job: one.job.title,
                    }),
              })}
            />
          ))}
        </Lane>
      </div>
    </div>
  );
}

export const take: OperatorTake = {
  name: "board",
  Queue,
};
