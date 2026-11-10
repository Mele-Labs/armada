// The Operator's state in this window: seeded by the mock scenario, moved by the panel's own presses. No Fleet route serves or
// takes any of it yet, so a press here changes this window alone.

import { createContext, useContext, useEffect, useMemo, useReducer } from "react";
import type { ReactNode } from "react";

import { offerTitle } from "./model";
import type { OperatorMessage, OperatorSeed } from "./model";

export type OperatorState = OperatorSeed & {
  messages: readonly OperatorMessage[];
  /** His words it is answering now: Discussion draws the thread's working mark until the answer is in. */
  thinking?: string;
};

export type Act =
  | {
      /** Start now, on a Job held to start. */
      kind: "start";
      line: string;
    }
  | {
      kind: "on";
      on: boolean;
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
      /** The mock's answer to the words it is thinking over, a moment after they were sent. */
      kind: "answer";
    }
  | {
      /** The Job a message offered: started in `repository`, or left where null. */
      kind: "offer";
      message: string;
      repository: string | null;
    };

let made = 0;
const fresh = (prefix: string) => {
  made += 1;
  return `${prefix}-${made}`;
};

const scriptFor = (state: OperatorState, words: string) => {
  const heard = words.toLowerCase();
  return state.talk.find((one) => heard.includes(one.asked.toLowerCase()));
};

function answered(state: OperatorState, words: string): OperatorState {
  const script = scriptFor(state, words);
  return {
    ...state,
    messages: [
      ...state.messages,
      {
        id: fresh("said"),
        from: "operator",
        at: new Date().toISOString(),
        text: script?.said ?? "This is the mock, so only the walk's words get an answer.",
      },
      // A Job it offers stands under its words, as Fleet's answer is drawn: its own message, so starting it leaves the words.
      ...(script?.offers === undefined
        ? []
        : [
            {
              id: fresh("offer"),
              from: "operator" as const,
              text: "",
              at: new Date().toISOString(),
              offer: {
                brief: script.offers,
                in: state.served ?? [],
              },
            },
          ]),
    ],
  };
}

function reduce(state: OperatorState, act: Act): OperatorState {
  switch (act.kind) {
    case "start":
      return {
        ...state,
        line: state.line.filter((one) => one.id !== act.line),
      };
    case "on":
      return {
        ...state,
        on: act.on,
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
      return {
        ...state,
        messages: [
          ...state.messages,
          {
            id: fresh("asked"),
            from: "you",
            at: new Date().toISOString(),
            text:
              act.attached.length === 0
                ? words
                : `${words}${words === "" ? "" : " "}Attached: ${act.attached.join(", ")}`,
          },
        ],
        // A call takes a moment: thinking first, then the answer.
        thinking: words,
      };
    }
    case "answer": {
      const { thinking: words, ...rest } = state;
      return words === undefined ? state : answered(rest, words);
    }
    case "clear":
      return {
        ...state,
        messages: [],
      };
    case "offer":
      return {
        ...state,
        messages: state.messages.map((one) => {
          if (one.id !== act.message || one.offer === undefined) return one;
          if (act.repository === null)
            return {
              ...one,
              offer: {
                ...one.offer,
                left: true,
              },
            };
          const title = offerTitle(one.offer.brief);
          return {
            ...one,
            text: `Started Job ${title} in ${act.repository}. Its answer is in its own thread.`,
            opens: {
              session: fresh("offered"),
              title,
            },
          };
        }),
      };
  }
}

export type OperatorStore = {
  state: OperatorState;
  act: (act: Act) => void;
};

const Held = createContext<OperatorStore | null>(null);

const THINKING_MS = 2500;

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
  // A call takes a moment, so the mock answers a beat after his words, with the working mark between.
  useEffect(() => {
    if (state.thinking === undefined) return;
    const answer = setTimeout(
      () =>
        act({
          kind: "answer",
        }),
      THINKING_MS,
    );
    return () => clearTimeout(answer);
  }, [
    state.thinking,
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
