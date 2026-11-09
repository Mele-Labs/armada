// A Job opened from the Operator: the Job's whole page laid over the Operator, which stays where it was behind it, from the sketch he
// approved on 9 Nov. Back or Esc closes it onto the same tab and scroll; Previous and Next step along the lane its card is in; Open
// as page leaves for the Job's own page, as Open Job did before.

import { useEffect, useRef, useSyncExternalStore } from "react";
import { ArrowDown, ArrowLeft, ArrowUp, ExternalLink, Square } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";
import { askToCloseOperator } from "@armada/shell";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { select } from "../cockpit/deck/store";
import { bringWalkForward } from "../job-walk/store";
import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import type { OperatorState } from "./store";

let over: string | null = null;
// Where focus was when the Job opened, so closing it puts him back on the card he pressed.
let opener: HTMLElement | null = null;
const listeners = new Set<() => void>();
const changed = () => {
  for (const one of listeners) one();
};
const subscribe = (listen: () => void) => {
  listeners.add(listen);
  return () => listeners.delete(listen);
};

export function openJobOver(id: string): void {
  if (over === null && document.activeElement instanceof HTMLElement)
    opener = document.activeElement;
  over = id;
  changed();
}

/** A Job's walk in front, over the Operator. `walk` is the page's address, which names the walk. */
export function openJobWalkOver(id: string, walk: string): void {
  bringWalkForward(id, walk);
  openJobOver(id);
}

export function closeJobOver(): void {
  if (over === null) return;
  over = null;
  changed();
  if (opener?.isConnected === true) opener.focus();
  opener = null;
}

/** Leaves the Operator for the Job's own page. */
export function openJobPage(id: string): void {
  closeJobOver();
  select({
    kind: "session",
    key: `session:${id}`,
  });
  askToCloseOperator();
}

const useJobOver = (): string | null => useSyncExternalStore(subscribe, () => over);

type Card = {
  session: string;
  title: string;
  repository?: string;
  tag?: string;
};

/** The board's lanes as Jobs in order, as `takes/board.tsx`'s Queue lays them: Now, Up next, Needs you, Done. */
function lanesOf(state: OperatorState): readonly (readonly Card[])[] {
  const turns = state.turns ?? [];
  const raises = state.raises ?? [];
  const open = turns.filter((one) => one.done === undefined);
  const pending = new Set(
    open.flatMap((one) =>
      one.raise === undefined
        ? []
        : [
            one.raise,
          ],
    ),
  );
  const ofTurn = (turn: (typeof turns)[number]): Card[] => {
    const raise = raises.find((one) => one.id === turn.raise);
    const job = turn.job ?? raise?.job;
    if (job === undefined) return [];
    const repository = raise?.repository ?? turn.start?.repository ?? turn.repository;
    const tag = turn.tag ?? raise?.tag;
    return [
      {
        session: job.session,
        title: job.title,
        ...(repository === undefined
          ? {}
          : {
              repository,
            }),
        ...(tag === undefined
          ? {}
          : {
              tag,
            }),
      },
    ];
  };
  const ofRaise = (raise: (typeof raises)[number]): Card[] =>
    raise.job === undefined
      ? []
      : [
          {
            session: raise.job.session,
            title: raise.job.title,
            repository: raise.repository,
            ...(raise.tag === undefined
              ? {}
              : {
                  tag: raise.tag,
                }),
          },
        ];
  const [onIt, ...next] = open;
  return [
    onIt === undefined ? [] : ofTurn(onIt),
    next.flatMap(ofTurn),
    raises.filter((one) => one.settled === undefined && one.toDiscuss !== true).flatMap(ofRaise),
    [
      ...raises.filter((one) => one.settled !== undefined && !pending.has(one.id)).flatMap(ofRaise),
      ...turns.filter((one) => one.done !== undefined).flatMap(ofTurn),
    ],
  ];
}

const editable = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable || target.closest("input, textarea, select") !== null);

const HELD = {
  held: {
    state: "none",
  },
  onWant: () => {},
} as const;

/** The open Job over the Operator, or nothing. */
export function JobOver({ state }: { state: OperatorState }) {
  const id = useJobOver();
  const session = useSessions().find((one) => one.id === id);
  const draft = useSessionsDraft();
  const back = useRef<HTMLButtonElement>(null);
  const layer = useRef<HTMLDivElement>(null);

  const lane = lanesOf(state)
    .map((cards) =>
      cards.filter((one, at) => cards.findIndex((other) => other.session === one.session) === at),
    )
    .find((cards) => cards.some((one) => one.session === id));
  const at = lane?.findIndex((one) => one.session === id) ?? -1;
  const card = lane?.[at];
  const previous = lane?.[at - 1];
  const next = lane?.[at + 1];

  useEffect(() => {
    if (id !== null) back.current?.focus();
  }, [
    id,
  ]);

  // Esc goes back, unless something inside the Job holds it: a menu, a dialog or a layer over its thread.
  useEffect(() => {
    if (id === null) return;
    const listen = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || popupIsOpen()) return;
      if (layer.current?.querySelector('[data-keeps-escape], [role="dialog"]') != null) return;
      event.preventDefault();
      closeJobOver();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [
    id,
  ]);

  if (id === null || session === undefined || draft === undefined) return null;
  const step = (to: Card | undefined) => {
    if (to !== undefined) openJobOver(to.session);
  };
  const place = [
    card?.repository,
    card?.tag,
  ]
    .filter(Boolean)
    .join(" · ");
  return (
    <div className="armada-operator__over" data-keeps-escape="">
      <div className="armada-operator__over-scrim" onClick={closeJobOver} aria-hidden />
      <div
        ref={layer}
        className="armada-operator__over-job"
        role="region"
        aria-label={`Job over the Operator: ${session.title ?? card?.title ?? session.id}`}
        onKeyDown={(event) => {
          if (!event.altKey || editable(event.target)) return;
          if (event.key === "ArrowUp" && previous !== undefined) {
            event.preventDefault();
            step(previous);
          } else if (event.key === "ArrowDown" && next !== undefined) {
            event.preventDefault();
            step(next);
          }
        }}
      >
        <header className="armada-operator__over-bar">
          <Button ref={back} size="sm" variant="ghost" onClick={closeJobOver}>
            <ArrowLeft size={14} strokeWidth={2} aria-hidden />
            Back to Operator
            <Kbd>Esc</Kbd>
          </Button>
          <div className="armada-operator__over-title">
            <span className="armada-operator__over-name">{session.title ?? card?.title}</span>
            {place === "" ? null : <span className="armada-operator__over-place">{place}</span>}
          </div>
          {lane === undefined ? null : (
            <div className="armada-operator__over-step">
              <Tooltip
                label={
                  previous === undefined ? "First in its lane" : `Previous: ${previous.title}, ⌥↑`
                }
              >
                <Button
                  size="sm"
                  variant="ghost"
                  iconOnly
                  aria-label="Previous Job in its lane"
                  disabled={previous === undefined}
                  onClick={() => step(previous)}
                >
                  <ArrowUp size={14} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
              <Tooltip label={next === undefined ? "Last in its lane" : `Next: ${next.title}, ⌥↓`}>
                <Button
                  size="sm"
                  variant="ghost"
                  iconOnly
                  aria-label="Next Job in its lane"
                  disabled={next === undefined}
                  onClick={() => step(next)}
                >
                  <ArrowDown size={14} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </div>
          )}
          <Button size="sm" variant="secondary" onClick={() => openJobPage(session.id)}>
            <ExternalLink size={14} strokeWidth={2} aria-hidden />
            Open as page
          </Button>
        </header>
        <section
          className="deck-pane armada-operator__over-page"
          data-frame=""
          aria-label={`Job: ${session.title ?? session.id}`}
          {...{
            [OPEN_SESSION_ATTRIBUTE]: session.id,
          }}
        >
          <SessionView
            session={session}
            goes={{
              onGoTo: () => {},
              onOpenLink: () => {},
              onOpenCheck: () => {},
              onOpenLoadout: () => {},
            }}
            onOpen={askToOpenSession}
            held={HELD}
            deck={{
              ledger: null,
              back: null,
              acts:
                draft.stop === undefined
                  ? []
                  : [
                      {
                        id: "stop",
                        label: "Stop",
                        icon: Square,
                        run: () => draft.stop?.(session.id),
                      },
                    ],
            }}
          />
        </section>
      </div>
    </div>
  );
}
