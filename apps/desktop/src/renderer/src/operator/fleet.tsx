// The Operator drawn from Fleet: its talk, and whether it is switched on. A press goes to Fleet's route for it; what this window keeps
// is only what he said since the talk was last read, a Job it offered, and what he cleared from view.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { OperatorTalk, Outcome, RepositorySummary } from "@armada/protocol";
import { jobIdOf } from "@armada/shell";

import type { BridgeState } from "../../../shared/bridge";
import { stateOf } from "../state-feed";
import { saveSetting, useSetting } from "../use-setting";
import type { OperatorMessage } from "./model";
import { OperatorHeld } from "./store";
import type { Act, OperatorState } from "./store";
import { offerTitle } from "./model";

/** A refusal in words, for Discussion. */
function refusedSaid(outcome: Outcome): string {
  if (outcome.ok) return "";
  if (outcome.why === "not_connected") return "Fleet is not connected.";
  if (outcome.why !== "refused") return "Fleet did not answer.";
  const { code, message } = outcome.error;
  if (code === "fleet.operator_off")
    return "I'm switched off, so I can't answer. Switch me on in Settings.";
  if (code === "fleet.operator_ceiling" || code === "fleet.operator_paused")
    return "I'm paused: your week's plan is past the share set in Settings.";
  if (code === "fleet.operator_said_nothing") return "I had nothing to say to that.";
  return message;
}

type Kept = {
  /** Talk before this time was cleared from Discussion. Fleet keeps it, for the Operator to read. */
  cleared?: string;
  /** His words not yet in what Fleet's talk read, and anything said back that Fleet keeps nowhere. */
  said: readonly OperatorMessage[];
  /** His last words while Fleet's call on them runs: Discussion draws the working mark. */
  thinking?: string;
};

/**
 * A Job started on `brief`. The start only names it: Fleet reads `request` for the title, model and effort and sends
 * the agent nothing, so the brief goes again as its first message, which is what sets it working.
 */
export async function startOnBrief(title: string, root: string, brief: string) {
  const started = await window.armada.startSession(title, root, brief);
  if (!started.ok) return started;
  const sent = await window.armada.sendSessionMessage({
    session_id: started.value.id,
    text: brief,
  });
  return sent.ok ? started : sent;
}

let made = 0;
const fresh = (prefix: string) => {
  made += 1;
  return `${prefix}-${made}`;
};

/** The Operator's store for a window served by Fleet. */
export function OperatorFromFleet({ children }: { children: ReactNode }) {
  const [repositories, setRepositories] = useState<readonly RepositorySummary[]>([]);
  const [lines, setLines] = useState<OperatorTalk["lines"]>([]);
  const [kept, setKept] = useState<Kept>({
    said: [],
  });
  const switched = useSetting("features.operator");

  useEffect(() => {
    const source = stateOf(window.armada);
    let on = true;
    const take = (state: BridgeState) => {
      if (!on) return;
      setRepositories(state.holds.repositories ?? []);
    };
    void source.state().then(take);
    const off = source.subscribe(take);
    return () => {
      on = false;
      off();
    };
  }, []);

  const readTalk = useCallback(async () => {
    const read = await window.armada.readOperatorTalk();
    if (!read.ok) return;
    setLines(read.talk.lines);
    // What Fleet now holds of his words is drawn from its read.
    setKept((was) => ({
      ...was,
      said: was.said.filter((one) => one.from === "operator"),
    }));
  }, []);
  useEffect(() => {
    void readTalk();
  }, [
    readTalk,
  ]);

  const state = useMemo((): OperatorState => {
    const talked: OperatorMessage[] = lines.map((one, at) => ({
      id: `line-${at}-${one.at}`,
      from: one.from_person ? "you" : "operator",
      text: one.text,
      at: one.at,
    }));
    const messages = [
      ...talked,
      ...kept.said,
    ]
      .filter((one) => kept.cleared === undefined || (one.at ?? "") > kept.cleared)
      .sort((a, b) => (a.at ?? "").localeCompare(b.at ?? ""));
    return {
      on: switched === true,
      line: [],
      reads: [],
      ties: [],
      talk: [],
      messages,
      ...(kept.thinking === undefined
        ? {}
        : {
            thinking: kept.thinking,
          }),
    };
  }, [
    lines,
    kept,
    switched,
  ]);

  // Where a Job the Operator offers may be started: every repository Fleet serves.
  const served = useMemo(
    () => repositories.flatMap((one) => jobIdOf(one) ?? []),
    [
      repositories,
    ],
  );

  /** His words to the Operator: drawn at once, and answered or refused in the talk. */
  const send = useCallback(
    (words: string) => {
      setKept((was) => ({
        ...was,
        said: [
          ...was.said,
          {
            id: fresh("asked"),
            from: "you",
            text: words,
            at: new Date().toISOString(),
          },
        ],
        thinking: words,
      }));
      // The working mark stays until what it said is drawn, or its refusal.
      const answeredNow = () =>
        setKept(({ thinking, ...was }) =>
          thinking === words
            ? was
            : {
                ...was,
                ...(thinking === undefined
                  ? {}
                  : {
                      thinking,
                    }),
              },
        );
      void window.armada.talkToOperator(words).then(async (answered) => {
        const offers = answered.ok ? answered.said.job : undefined;
        // Fleet keeps the words it said, not the Job it offered, so the offer is held here under them.
        if (offers !== undefined && offers.trim() !== "")
          setKept((was) => ({
            ...was,
            said: [
              ...was.said,
              {
                id: fresh("offer"),
                from: "operator",
                text: "",
                at: new Date().toISOString(),
                offer: {
                  brief: offers.trim(),
                  in: served,
                },
              },
            ],
          }));
        if (answered.ok) {
          await readTalk();
          return answeredNow();
        }
        setKept((was) => ({
          ...was,
          said: [
            ...was.said,
            {
              id: fresh("refused"),
              from: "operator",
              text: refusedSaid(answered.outcome),
              at: new Date().toISOString(),
            },
          ],
        }));
        answeredNow();
      });
    },
    [
      readTalk,
      served,
    ],
  );

  const act = useCallback(
    (one: Act) => {
      switch (one.kind) {
        case "clear":
          setKept((was) => ({
            ...was,
            cleared: new Date().toISOString(),
            said: [],
          }));
          return;
        case "offer": {
          const offered = kept.said.find((each) => each.id === one.message)?.offer;
          if (offered === undefined) return;
          const settle = (change: Partial<OperatorMessage>) =>
            setKept((was) => ({
              ...was,
              said: was.said.map((each) =>
                each.id === one.message
                  ? {
                      ...each,
                      ...change,
                    }
                  : each,
              ),
            }));
          if (one.repository === null)
            return settle({
              offer: {
                ...offered,
                left: true,
              },
            });
          const title = offerTitle(offered.brief);
          const root = repositories.find((each) => jobIdOf(each) === one.repository)?.root;
          if (root === undefined)
            return settle({
              text: `${title} did not start: ${one.repository} is not served here.`,
            });
          void startOnBrief(title, root, offered.brief).then((started) =>
            started.ok
              ? settle({
                  text: `Started Job ${title} in ${one.repository}. Its answer is in its own thread.`,
                  opens: {
                    session: started.value.id,
                    title,
                  },
                })
              : settle({
                  text: `${title} did not start: ${refusedSaid(started.outcome)}`,
                }),
          );
          return;
        }
        case "say": {
          const words = one.text.trim();
          if (words === "") return;
          send(words);
          return;
        }
        case "on":
          void saveSetting("features.operator", one.on);
          return;
        default:
          // `start`, `answered` and `answer` belong to the mock's own Jobs: Fleet serves none of them to this page.
          return;
      }
    },
    [
      kept.said,
      repositories,
      send,
    ],
  );

  const store = useMemo(
    () => ({
      state,
      act,
    }),
    [
      state,
      act,
    ],
  );
  return <OperatorHeld value={store}>{children}</OperatorHeld>;
}
