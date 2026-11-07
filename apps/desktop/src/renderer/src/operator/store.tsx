// The Operator's state in this window: seeded by the mock scenario, moved by the panel's own presses. No Fleet route serves or
// takes any of it yet, so a press here changes this window alone.

import { createContext, useContext, useEffect, useMemo, useReducer } from "react";
import type { ReactNode } from "react";

import { DID_SAID, waitSaid } from "./model";
import type {
  OperatorLineRow,
  OperatorLogRow,
  OperatorMessage,
  OperatorPosition,
  OperatorRole,
  OperatorSeed,
  OperatorTurn,
} from "./model";

export type OperatorState = OperatorSeed & {
  messages: readonly OperatorMessage[];
  /** The Waiting on you row being discussed: its card is in the conversation, and words go to it. */
  open?: string;
};

export type Act =
  | {
      kind: "start";
      line: string;
      by: "press" | "talk";
    }
  | {
      kind: "move";
      line: string;
      before: string | null;
    }
  | {
      kind: "change";
      log: string;
      way: string;
    }
  | {
      kind: "position";
      role: OperatorRole;
      position: OperatorPosition;
    }
  | {
      kind: "on";
      on: boolean;
    }
  | {
      kind: "pause";
      share: number;
    }
  | {
      kind: "unorder";
      order: string;
    }
  | {
      kind: "answered";
      session: string;
    }
  | {
      kind: "say";
      text: string;
      /** The names of the files and sketches sent with it. */
      attached: readonly string[];
    }
  | {
      kind: "clear";
    }
  | {
      kind: "raise";
      raise: string;
      way: string;
    }
  | {
      kind: "order";
      message: string;
      keep: boolean;
    }
  | {
      kind: "repository";
      name: string;
      role: OperatorRole;
      /** Null follows the machine again. */
      position: OperatorPosition | null;
    }
  | {
      kind: "add-repository";
      name: string;
    }
  | {
      kind: "model";
      model: string;
    }
  | {
      kind: "turn";
    }
  | {
      kind: "move-turn";
      turn: string;
      before: string | null;
    }
  | {
      kind: "move-raise";
      raise: string;
      before: string | null;
    }
  | {
      kind: "start-turn";
      turn: string;
    }
  | {
      /** Now, or in its turn where the Operator is busy. */
      kind: "discuss";
      raise: string;
      now: boolean;
    }
  | {
      /** Stop discussing it. Its row is still waiting. */
      kind: "not-now";
    }
  | {
      /** Starts a Job about the item: one that reads the code and answers it, or one that reviews its Job's work. */
      kind: "dispatch";
      raise: string;
      what: "investigate" | "review";
    };

let made = 0;
const fresh = (prefix: string) => {
  made += 1;
  return `${prefix}-${made}`;
};

function logged(state: OperatorState, row: Omit<OperatorLogRow, "id" | "at">): OperatorState {
  return {
    ...state,
    log: [
      {
        id: fresh("you"),
        at: new Date().toISOString(),
        ...row,
      },
      ...state.log,
    ],
  };
}

const YOU = {
  kind: "you",
  did: "corrected",
} as const;

function started(state: OperatorState, line: OperatorLineRow, by: "press" | "talk"): OperatorState {
  const without: OperatorState = {
    ...state,
    line: state.line.filter((one) => one.id !== line.id),
    // A held row's Start now goes with the hold it overrode.
    log: state.log.map((row) => {
      if (row.line !== line.id) return row;
      const { line: _gone, ...rest } = row;
      return rest;
    }),
  };
  return logged(without, {
    ...(line.repository === undefined
      ? {}
      : {
          repository: line.repository,
        }),
    from: YOU,
    item: line.title,
    did: "started",
    why:
      by === "talk"
        ? "You said start it anyway. Kept as your correction"
        : `Start now. It waited for ${waitSaid(line.waits)}`,
  });
}

const scriptFor = (state: OperatorState, words: string) => {
  const heard = words.toLowerCase();
  return state.talk.find((one) => heard.includes(one.asked.toLowerCase()));
};

function answered(state: OperatorState, words: string): OperatorState {
  const script = scriptFor(state, words);
  const said: OperatorState = {
    ...state,
    messages: [
      ...state.messages,
      {
        id: fresh("said"),
        from: "operator",
        text: script?.said ?? "Nothing in the Log or Waiting to start answers that.",
        ...(script?.order === undefined
          ? {}
          : {
              order: script.order,
            }),
      },
    ],
  };
  const line =
    script?.starts === undefined ? undefined : said.line.find((one) => one.id === script.starts);
  if (line !== undefined) return started(said, line, "talk");
  const puts = script?.puts;
  if (puts === undefined) return said;
  const open = said.turns?.filter((one) => one.done === undefined) ?? [];
  const went = movedIn(open, puts.turn, open[1]?.id ?? null, 1);
  const list = went?.list ?? open;
  return {
    ...said,
    turns: [
      ...list.map((one) =>
        one.id === puts.turn
          ? {
              ...one,
              put: puts.line,
            }
          : one,
      ),
      ...(said.turns?.filter((one) => one.done !== undefined) ?? []),
    ],
  };
}

/** `id` moved before `before`, or to the end: the list, and where it went from and to. Nothing where it stays put. */
function movedIn<
  T extends {
    id: string;
  },
>(
  list: readonly T[],
  id: string,
  before: string | null,
  floor: number,
): {
  list: T[];
  from: number;
  to: number;
} | null {
  const from = list.findIndex((one) => one.id === id);
  if (from < floor || id === before) return null;
  const rest = list.filter((one) => one.id !== id);
  const found = before === null ? rest.length : rest.findIndex((one) => one.id === before);
  if (found < 0) return null;
  const to = Math.max(found, floor);
  if (to === from) return null;
  return {
    list: [
      ...rest.slice(0, to),
      list[from]!,
      ...rest.slice(to),
    ],
    from,
    to,
  };
}

const openTurns = (state: OperatorState): readonly OperatorTurn[] =>
  (state.turns ?? []).filter((one) => one.done === undefined);

const doneTurns = (state: OperatorState): readonly OperatorTurn[] =>
  (state.turns ?? []).filter((one) => one.done !== undefined);

/** What the Operator says to words about the item being discussed. What it would now do becomes the item's first press. */
function answeredOnItem(state: OperatorState, raise: string, words: string): OperatorState {
  const heard = words.toLowerCase();
  const script = state.raises
    ?.find((one) => one.id === raise)
    ?.talk?.replies.find((one) => heard.includes(one.asked.toLowerCase()));
  const send = script?.send;
  return {
    ...state,
    raises:
      send === undefined
        ? state.raises
        : state.raises?.map((one) =>
            one.id === raise
              ? {
                  ...one,
                  read: send.read,
                  ways: [
                    send.press,
                    ...one.ways.slice(1),
                  ],
                  results: {
                    ...one.results,
                    [send.press]: send.result,
                  },
                }
              : one,
          ),
    messages: [
      ...state.messages,
      {
        id: fresh("said"),
        from: "operator",
        text: script?.said ?? "This is the mock, so only the walk's words get an answer.",
      },
    ],
  };
}

/** The item comes up in the conversation as a card, and what the Operator makes of it the first time. */
function discussed(state: OperatorState, raise: string): OperatorState {
  const raised = state.raises?.find((one) => one.id === raise);
  if (raised === undefined) return state;
  const before = state.messages.some((one) => one.card === raise);
  return {
    ...state,
    open: raise,
    turns: state.turns?.filter((one) => !(one.discuss === true && one.raise === raise)),
    raises: state.raises?.map((one) =>
      one.id === raise
        ? {
            ...one,
            toDiscuss: false,
          }
        : one,
    ),
    messages: [
      ...state.messages,
      {
        id: fresh("card"),
        from: "operator",
        text: "",
        card: raise,
      },
      ...(before
        ? []
        : [
            {
              id: fresh("said"),
              from: "operator" as const,
              text: raised.talk?.first ?? raised.read,
            },
          ]),
    ],
  };
}

function reduce(state: OperatorState, act: Act): OperatorState {
  switch (act.kind) {
    case "start": {
      const line = state.line.find((one) => one.id === act.line);
      return line === undefined ? state : started(state, line, act.by);
    }
    case "move": {
      const moving = state.line.find((one) => one.id === act.line);
      if (moving === undefined || act.line === act.before) return state;
      const rest = state.line.filter((one) => one.id !== act.line);
      const at = act.before === null ? rest.length : rest.findIndex((one) => one.id === act.before);
      const line = [
        ...rest.slice(0, at),
        moving,
        ...rest.slice(at),
      ];
      const after = line[at + 1];
      return logged(
        {
          ...state,
          line,
        },
        {
          ...(moving.repository === undefined
            ? {}
            : {
                repository: moving.repository,
              }),
          from: YOU,
          item: moving.title,
          did: "moved",
          why:
            after === undefined
              ? "To the end of Waiting to start. Kept as your correction"
              : `Ahead of ${after.title}. Kept as your correction`,
        },
      );
    }
    case "change": {
      const ruled = state.log.find((row) => row.id === act.log);
      if (ruled === undefined) return state;
      return logged(
        {
          ...state,
          log: state.log.map((row) =>
            row.id === act.log
              ? {
                  ...row,
                  changed: act.way,
                }
              : row,
          ),
        },
        {
          ...(ruled.repository === undefined
            ? {}
            : {
                repository: ruled.repository,
              }),
          from: YOU,
          item: ruled.item,
          did: "overruled",
          changed: act.way,
          why: `${DID_SAID[ruled.did]} became ${act.way}. Kept as your correction`,
        },
      );
    }
    case "position":
      return {
        ...state,
        positions: {
          ...state.positions,
          [act.role]: act.position,
        },
      };
    case "on":
      return {
        ...state,
        on: act.on,
      };
    case "pause":
      return {
        ...state,
        pause: act.share,
      };
    case "unorder":
      return {
        ...state,
        orders: state.orders.filter((one) => one.id !== act.order),
      };
    case "answered":
      // Your answer wins: the ruling is dropped when it returns.
      return {
        ...state,
        reads: state.reads.filter((one) => one.session !== act.session),
      };
    case "say": {
      const words = act.text.trim();
      if (words === "" && act.attached.length === 0) return state;
      const asked: OperatorMessage = {
        id: fresh("asked"),
        from: "you",
        at: new Date().toISOString(),
        text:
          act.attached.length === 0
            ? words
            : `${words}${words === "" ? "" : " "}Attached: ${act.attached.join(", ")}`,
      };
      const heard: OperatorState = {
        ...state,
        messages: [
          ...state.messages,
          asked,
        ],
      };
      // It is discussing this with you, so it answers you ahead of its list.
      if (state.open !== undefined) return answeredOnItem(heard, state.open, words);
      const turns = openTurns(state);
      // Words that set the order are acted on at once: they cannot wait in the list they change.
      if (turns.length === 0 || scriptFor(state, words)?.puts !== undefined)
        return answered(heard, words);
      return {
        ...heard,
        turns: [
          ...turns,
          {
            id: asked.id,
            text: asked.text,
            message: asked.id,
          },
          ...doneTurns(state),
        ],
      };
    }
    case "turn": {
      const [done, ...rest] = openTurns(state);
      if (done === undefined) return state;
      // A row that came to something stays, newest first below what is open, saying what.
      const came = done.result ?? done.finish;
      const moved: OperatorState = {
        ...state,
        turns: [
          ...rest,
          ...(came === undefined || done.discuss === true
            ? []
            : [
                {
                  ...done,
                  done: came,
                },
              ]),
          ...doneTurns(state),
        ],
        raises:
          done.result === undefined
            ? state.raises
            : state.raises?.map((one) =>
                one.id === done.raise
                  ? {
                      ...one,
                      settled: done.result,
                    }
                  : one,
              ),
      };
      const said = done.message === undefined ? moved : answered(moved, done.text);
      // What he asked to discuss comes up when it reaches the top, unless he is in another with it.
      const next = rest[0];
      return next?.discuss === true && next.raise !== undefined && said.open === undefined
        ? discussed(said, next.raise)
        : said;
    }
    case "move-turn": {
      // The row being read stays first: a row dropped above it comes next.
      const went = movedIn(openTurns(state), act.turn, act.before, 1);
      if (went === null) return state;
      const way: "up" | "down" = went.to < went.from ? "up" : "down";
      const row = went.list[went.to]!;
      return logged(
        {
          ...state,
          turns: [
            ...went.list.map((one) =>
              one.id === act.turn
                ? {
                    ...one,
                    moved: way,
                  }
                : one,
            ),
            ...doneTurns(state),
          ],
        },
        {
          ...(row.repository === undefined
            ? {}
            : {
                repository: row.repository,
              }),
          from: YOU,
          item: row.text,
          did: "moved",
          why: `You moved it ${way} in Working through. Kept as your correction`,
        },
      );
    }
    case "move-raise": {
      const went = movedIn(state.raises ?? [], act.raise, act.before, 0);
      return went === null
        ? state
        : {
            ...state,
            raises: went.list,
          };
    }
    case "start-turn": {
      const row = state.turns?.find((one) => one.id === act.turn);
      if (row === undefined) return state;
      return logged(
        {
          ...state,
          turns: [
            ...openTurns(state).filter((one) => one.id !== act.turn),
            ...doneTurns(state),
            {
              ...row,
              done: "You started it",
            },
          ],
        },
        {
          from: YOU,
          item: row.text,
          did: "started",
          why: "Start now",
        },
      );
    }
    case "discuss": {
      const raised = state.raises?.find((one) => one.id === act.raise);
      if (raised === undefined) return state;
      if (act.now || openTurns(state).length === 0) return discussed(state, act.raise);
      return {
        ...state,
        raises: state.raises?.map((one) =>
          one.id === act.raise
            ? {
                ...one,
                toDiscuss: true,
              }
            : one,
        ),
        turns: [
          ...openTurns(state),
          {
            id: fresh("discuss"),
            text: raised.item,
            about: "It brings this up in Discussion when it gets here",
            raise: act.raise,
            discuss: true,
          },
          ...doneTurns(state),
        ],
      };
    }
    case "not-now": {
      const { open: _closed, ...rest } = state;
      return rest;
    }
    case "dispatch": {
      const raised = state.raises?.find((one) => one.id === act.raise);
      if (raised?.job === undefined) return state;
      const title = `${act.what === "review" ? "Review" : "Investigate"}: ${raised.job.title}`;
      return {
        ...state,
        messages: [
          ...state.messages,
          {
            id: fresh("started"),
            from: "operator",
            text: `Started Job ${title}. Its answer is in its own thread.`,
            at: new Date().toISOString(),
            opens: {
              session: fresh("job"),
              title,
            },
          },
        ],
      };
    }
    case "clear":
      // A message still queued stays: it has not been answered.
      return {
        ...state,
        messages: state.messages.filter(
          (one) =>
            (one.card !== undefined && one.card === state.open) ||
            state.turns?.some((turn) => turn.message === one.id),
        ),
      };
    case "raise": {
      const raised = state.raises?.find((one) => one.id === act.raise);
      if (raised === undefined) return state;
      const result = raised.results?.[act.way] ?? `You pressed ${act.way}`;
      const queues = raised.queues?.includes(act.way) === true;
      const { open, ...closed } = state;
      const left = open === act.raise ? closed : state;
      return logged(
        {
          ...left,
          // Settled while being discussed, it says so where the talk ended.
          messages:
            open === act.raise
              ? [
                  ...left.messages,
                  {
                    id: fresh("card"),
                    from: "operator",
                    text: "",
                    card: act.raise,
                    ends: true,
                  },
                ]
              : left.messages,
          turns: queues
            ? [
                ...openTurns(left).filter((one) => one.raise !== act.raise),
                {
                  id: fresh("carry"),
                  text: `${act.way}: ${raised.item}`,
                  about: "You pressed it. It does this in its turn",
                  raise: act.raise,
                  result,
                },
                ...doneTurns(left),
              ]
            : left.turns?.filter((one) => !(one.discuss === true && one.raise === act.raise)),
          raises:
            left.raises?.map((one) =>
              one.id === act.raise
                ? {
                    ...one,
                    toDiscuss: false,
                    settled: queues ? "In the Operator's Queue. It does this in its turn" : result,
                    settledBy: "You",
                    settledAt: new Date().toISOString(),
                  }
                : one,
            ) ?? [],
        },
        {
          from: YOU,
          item: raised.item,
          did: act.way === raised.ways[0] ? raised.does : "raised",
          why: `You pressed ${act.way}`,
          ...(raised.manifest === undefined
            ? {}
            : {
                repository: raised.manifest,
              }),
        },
      );
    }
    case "order": {
      const message = state.messages.find((one) => one.id === act.message);
      if (message?.order === undefined) return state;
      return {
        ...state,
        messages: state.messages.map((one) =>
          one.id === act.message
            ? {
                ...one,
                settled: act.keep ? "kept" : "left",
              }
            : one,
        ),
        orders: act.keep
          ? [
              ...state.orders,
              message.order,
            ]
          : state.orders,
      };
    }
    case "repository":
      return {
        ...state,
        repositories: (state.repositories ?? []).map((one) => {
          if (one.name !== act.name) return one;
          const { [act.role]: _was, ...rest } = one.positions;
          return {
            ...one,
            positions:
              act.position === null
                ? rest
                : {
                    ...rest,
                    [act.role]: act.position,
                  },
          };
        }),
      };
    case "add-repository":
      return {
        ...state,
        repositories: [
          ...(state.repositories ?? []),
          {
            name: act.name,
            positions: {},
          },
        ],
      };
    case "model":
      return {
        ...state,
        model: act.model,
      };
  }
}

export type OperatorStore = {
  state: OperatorState;
  act: (act: Act) => void;
};

const Held = createContext<OperatorStore | null>(null);

/** The Operator's state for this window, where the scenario seeds one. Without a seed nothing of the Operator is drawn. */
export function OperatorFrom({
  seed,
  children,
}: {
  seed: OperatorSeed | undefined;
  children: ReactNode;
}) {
  return seed === undefined ? <>{children}</> : <Seeded seed={seed}>{children}</Seeded>;
}

function Seeded({ seed, children }: { seed: OperatorSeed; children: ReactNode }) {
  const [state, act] = useReducer(reduce, seed, (first) => ({
    ...first,
    messages: [],
  }));
  useEffect(() => {
    seed.moves?.(() =>
      act({
        kind: "turn",
      }),
    );
  }, [
    seed,
  ]);
  const store = useMemo(
    () => ({
      state,
      act,
    }),
    [
      state,
    ],
  );
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

/** Where a store is made elsewhere, such as from Fleet (`fleet.tsx`). */
export const OperatorHeld = Held.Provider;

/** The Operator's store, or null where this window has none: no dock is drawn. */
export function useOperator(): OperatorStore | null {
  return useContext(Held);
}
