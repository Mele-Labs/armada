// The Operator's state in this window: seeded by the mock scenario, moved by the panel's own presses. No Fleet route serves or
// takes any of it yet, so a press here changes this window alone.

import { createContext, useContext, useMemo, useReducer } from "react";
import type { ReactNode } from "react";

import { waitSaid } from "./model";
import type {
  OperatorLineRow,
  OperatorLogRow,
  OperatorMessage,
  OperatorPosition,
  OperatorRole,
  OperatorSeed,
} from "./model";

export type OperatorState = OperatorSeed & {
  messages: readonly OperatorMessage[];
};

type Act =
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
      kind: "ceiling";
      calls: number;
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
    }
  | {
      kind: "clear";
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
    from: "You",
    item: line.title,
    did: "started",
    why:
      by === "talk"
        ? "You said start it anyway. Kept as your correction"
        : `Start now, over the wait for ${waitSaid(line.waits)}`,
  });
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
          from: "You",
          item: moving.title,
          did: "moved",
          why:
            after === undefined
              ? "To the end of the Line. Kept as your correction"
              : `Ahead of ${after.title}. Kept as your correction`,
        },
      );
    }
    case "change":
      return {
        ...state,
        log: state.log.map((row) =>
          row.id === act.log
            ? {
                ...row,
                changed: act.way,
              }
            : row,
        ),
      };
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
    case "ceiling":
      return {
        ...state,
        ceiling: act.calls,
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
      if (words === "") return state;
      const heard = words.toLowerCase();
      const script = state.talk.find((one) => heard.includes(one.asked.toLowerCase()));
      const said: OperatorState = {
        ...state,
        messages: [
          ...state.messages,
          {
            id: fresh("asked"),
            from: "you",
            text: words,
          },
          {
            id: fresh("said"),
            from: "operator",
            text: script?.said ?? "Nothing in the Log or the Line answers that.",
          },
        ],
      };
      const line =
        script?.starts === undefined
          ? undefined
          : said.line.find((one) => one.id === script.starts);
      return line === undefined ? said : started(said, line, "talk");
    }
    case "clear":
      return {
        ...state,
        messages: [],
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

/** The Operator's store, or null where this window has none: Helm's dock is drawn as it was. */
export function useOperator(): OperatorStore | null {
  return useContext(Held);
}
