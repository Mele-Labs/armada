// The Operator drawn from Fleet: the board it serves (what it is on, what waits for it, what it settled), every live Job's
// Needs you items, its talk, and its settings. A press goes to Fleet's route for it; what this window keeps is only what
// is open in Discussion, where an item came up there, and what he pressed here since it opened.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type {
  OperatorBoard,
  OperatorBoardItem,
  OperatorBoardKind,
  OperatorBoardSuggestion,
  OperatorTalk,
  Outcome,
  WaitingItem,
} from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../shared/bridge";
import { useSessionsEverywhere } from "../sessions-draft";
import { stateOf } from "../state-feed";
import { saveSetting, useSetting } from "../use-setting";
import type {
  OperatorMessage,
  OperatorPosition,
  OperatorRaise,
  OperatorRole,
  OperatorTurn,
} from "./model";
import { OperatorHeld } from "./store";
import type { Act, OperatorState } from "./store";
import { openJob } from "./takes/board";

const KIND_TAG: Readonly<Record<OperatorBoardKind, string>> = {
  start: "New Job",
  question: "Question",
  card: "Question",
  stall: "Quiet Job",
  report: "Report",
};

const SOURCE_TAG: Partial<Readonly<Record<WaitingItem["source"], string>>> = {
  operator_suggestion: "Suggestion",
  agent: "Question",
  ask_card: "Question",
  walk: "Walk",
  draft: "Draft",
  plan: "Plan",
  sketch: "Sketch",
  document: "Document",
  permission: "Permission",
};

const OPEN_JOB = "Open Job";

const ROLE_KEY: Readonly<Record<OperatorRole, string>> = {
  dispatch: "features.operatorDispatch",
  delegate: "features.operatorDelegate",
  answer: "features.operatorAnswer",
  file: "features.operatorFile",
};

const turnOf = (item: OperatorBoardItem): OperatorTurn => ({
  id: item.id,
  text: item.title,
  tag: KIND_TAG[item.kind],
  ...(item.repository === undefined
    ? {}
    : {
        repository: item.repository,
      }),
});

/** One Needs you item of a Job, as a card: the Operator's pick first where it read one. */
function raiseOf(session: Session, item: WaitingItem): OperatorRaise {
  const labels = (item.options ?? []).map((one) => one.label);
  const pick = item.operator_read?.pick;
  const ways =
    labels.length === 0
      ? [
          OPEN_JOB,
        ]
      : pick !== undefined && labels.includes(pick)
        ? [
            pick,
            ...labels.filter((one) => one !== pick),
          ]
        : labels;
  const title = session.title ?? session.address ?? "Job";
  const suggests = item.source === "operator_suggestion";
  return {
    id: `${session.id} ${item.id}`,
    suggests,
    repository: session.manifestId ?? "",
    ...(session.manifestId === undefined
      ? {}
      : {
          manifest: session.manifestId,
        }),
    item: suggests ? title : item.text,
    read: suggests ? item.text : (item.operator_read?.read ?? title),
    ways,
    does: "raised",
    tag: SOURCE_TAG[item.source] ?? "Needs you",
    job: {
      session: session.id,
      title,
    },
  };
}

/** A suggestion no Job holds, such as what to do with his note: Do it or Skip, and no words. */
function noteOf(suggestion: OperatorBoardSuggestion): OperatorRaise {
  const at = suggestion.text.indexOf(": ");
  return {
    id: suggestion.id,
    suggests: true,
    repository: "",
    item: at === -1 ? suggestion.text : suggestion.text.slice(0, at),
    read: at === -1 ? "" : suggestion.text.slice(at + 2),
    ways: suggestion.options.map((one) => one.label),
    does: "raised",
    tag: "Suggestion",
  };
}

const live = (session: Session) => session.dead === undefined && session.replacedBy === undefined;

const POSITIONS: readonly OperatorPosition[] = [
  "on",
  "suggest",
  "off",
];
const positionOf = (value: unknown): OperatorPosition =>
  POSITIONS.includes(value as OperatorPosition) ? (value as OperatorPosition) : "off";

/** A refusal in words, for Discussion. */
function refusedSaid(outcome: Outcome): string {
  const code = (
    outcome as {
      code?: string;
    }
  ).code;
  if (code === "fleet.operator_off") return "I'm switched off. Switch me on under Settings.";
  if (code === "fleet.operator_ceiling") return "I've used today's calls.";
  if (code === "fleet.operator_said_nothing") return "I had nothing to say to that.";
  if (outcome.ok === false && outcome.why === "not_connected") return "Fleet is not connected.";
  return "That did not reach me.";
}

type Kept = {
  /** The Needs you card open in Discussion. */
  open?: string;
  /** Where an item came up in Discussion, and where it was settled. */
  marks: readonly OperatorMessage[];
  /** His words not yet in what Fleet's talk read, and anything said back that Fleet keeps nowhere. */
  said: readonly OperatorMessage[];
  /** Cards he settled here, kept in Done after the item leaves its Job's list. */
  settled: readonly OperatorRaise[];
  /** Suggestions no Job holds that he pressed and Fleet has not answered: their outcome arrives in the board's Done. */
  pressing: readonly string[];
};

let made = 0;
const fresh = (prefix: string) => {
  made += 1;
  return `${prefix}-${made}`;
};

/** The Operator's store for a window served by Fleet. */
export function OperatorFromFleet({ children }: { children: ReactNode }) {
  const sessions = useSessionsEverywhere();
  const [board, setBoard] = useState<OperatorBoard | null>(null);
  const [lines, setLines] = useState<OperatorTalk["lines"]>([]);
  const [kept, setKept] = useState<Kept>({
    marks: [],
    said: [],
    settled: [],
    pressing: [],
  });
  const switched = useSetting("features.operator");
  const dispatch = useSetting("features.operatorDispatch");
  const delegate = useSetting("features.operatorDelegate");
  const answer = useSetting("features.operatorAnswer");
  const file = useSetting("features.operatorFile");
  const ceiling = useSetting("limits.operatorDailyCalls");
  const model = useSetting("models.operator");

  useEffect(() => {
    const source = stateOf(window.armada);
    let on = true;
    const take = (state: BridgeState) => on && setBoard(state.operatorBoard ?? null);
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

  const raises = useMemo(() => {
    const waiting = sessions
      .filter(live)
      .flatMap((session) => (session.waitingFor ?? []).map((item) => raiseOf(session, item)));
    const settledIds = new Set(kept.settled.map((one) => one.id));
    const notes = (board?.suggestions ?? [])
      .filter((one) => !kept.pressing.includes(one.id))
      .map(noteOf);
    return [
      ...waiting.filter((one) => !settledIds.has(one.id)),
      ...notes,
      ...kept.settled,
    ];
  }, [
    sessions,
    board,
    kept.settled,
    kept.pressing,
  ]);

  const state = useMemo((): OperatorState => {
    const turns: OperatorTurn[] = [
      ...(board?.now === undefined
        ? []
        : [
            turnOf(board.now),
          ]),
      ...(board?.next ?? []).map(turnOf),
      ...(board?.done ?? []).map((one) => ({
        ...turnOf(one),
        done: one.outcome,
      })),
    ];
    const talked: OperatorMessage[] = lines.map((one, at) => ({
      id: `line-${at}-${one.at}`,
      from: one.from_person ? "you" : "operator",
      text: one.text,
      at: one.at,
    }));
    const messages = [
      ...talked,
      ...kept.marks,
      ...kept.said,
    ].sort((a, b) => (a.at ?? "").localeCompare(b.at ?? ""));
    return {
      on: board?.on ?? switched === true,
      positions: {
        dispatch: positionOf(dispatch),
        delegate: positionOf(delegate),
        answer: positionOf(answer),
        file: positionOf(file),
      },
      ceiling: Number(ceiling ?? 100),
      ...(typeof model === "string"
        ? {
            model,
          }
        : {}),
      log: [],
      line: [],
      orders: [],
      reads: [],
      ties: [],
      talk: [],
      raises,
      turns,
      take: "board",
      discussesAtOnce: true,
      messages,
      ...(kept.open === undefined
        ? {}
        : {
            open: kept.open,
          }),
    };
  }, [
    board,
    lines,
    kept,
    raises,
    switched,
    dispatch,
    delegate,
    answer,
    file,
    ceiling,
    model,
  ]);

  const act = useCallback(
    (one: Act) => {
      const now = () => new Date().toISOString();
      const mark = (card: string, ends?: boolean): OperatorMessage => ({
        id: fresh("mark"),
        from: "operator",
        text: "",
        card,
        at: now(),
        ...(ends === true
          ? {
              ends,
            }
          : {}),
      });
      switch (one.kind) {
        case "move-turn":
          void window.armada.moveOnOperatorBoard(one.turn, one.before ?? undefined);
          return;
        case "discuss":
          setKept((was) => ({
            ...was,
            open: one.raise,
            marks: [
              ...was.marks,
              mark(one.raise),
            ],
          }));
          return;
        case "not-now":
          setKept((was) => {
            const { open: _open, ...rest } = was;
            return rest;
          });
          return;
        case "raise": {
          const raise = raises.find((each) => each.id === one.raise);
          if (raise !== undefined && raise.job === undefined) {
            const choice = raise.ways.indexOf(one.way);
            if (choice !== 0 && choice !== 1) return;
            const id = raise.id;
            setKept((was) => ({
              ...was,
              pressing: [
                ...was.pressing,
                id,
              ],
            }));
            void window.armada.answerOperatorSuggestion(id, choice).then((answered) => {
              setKept((was) => ({
                ...was,
                pressing: was.pressing.filter((each) => each !== id),
                ...(answered.ok
                  ? {}
                  : {
                      said: [
                        ...was.said,
                        {
                          id: fresh("refused"),
                          from: "operator" as const,
                          text: `${raise.item}: ${refusedSaid(answered.outcome)}`,
                          at: now(),
                        },
                      ],
                    }),
              }));
            });
            return;
          }
          if (raise?.job === undefined) return;
          if (one.way === OPEN_JOB) return openJob(raise.job.session);
          const [session, ...rest] = one.raise.split(" ");
          const item = rest.join(" ");
          const choice = sessions
            .find((each) => each.id === session)
            ?.waitingFor?.find((each) => each.id === item)
            ?.options?.findIndex((each) => each.label === one.way);
          if (choice === undefined || choice === -1) return openJob(raise.job.session);
          const outcome = `${raise.suggests ? "Done" : "Answered"}: ${one.way}`;
          setKept((was) => {
            const { open, ...rest } = was;
            return {
              ...rest,
              ...(open === undefined || open === one.raise
                ? {}
                : {
                    open,
                  }),
              marks:
                open === one.raise
                  ? [
                      ...was.marks,
                      mark(one.raise, true),
                    ]
                  : was.marks,
              settled: [
                {
                  ...raise,
                  settled: outcome,
                },
                ...was.settled,
              ],
            };
          });
          void window.armada.answerWaiting({
            session_id: session!,
            item_id: item,
            choice,
          });
          return;
        }
        case "say": {
          const words = one.text.trim();
          if (words === "") return;
          const about = kept.open?.split(" ").slice(1).join(" ");
          setKept((was) => ({
            ...was,
            said: [
              ...was.said,
              {
                id: fresh("asked"),
                from: "you",
                text: words,
                at: now(),
              },
            ],
          }));
          void window.armada.talkToOperator(words, about).then((answered) => {
            if (answered.ok) return readTalk();
            setKept((was) => ({
              ...was,
              said: [
                ...was.said,
                {
                  id: fresh("refused"),
                  from: "operator",
                  text: refusedSaid(answered.outcome),
                  at: now(),
                },
              ],
            }));
          });
          return;
        }
        case "on":
          void saveSetting("features.operator", one.on);
          return;
        case "position":
          void saveSetting(ROLE_KEY[one.role], one.position);
          return;
        case "ceiling":
          void saveSetting("limits.operatorDailyCalls", one.calls);
          return;
        case "model":
          void saveSetting("models.operator", one.model);
          return;
        default:
          // The mock's own acts (Waiting to start, the Log, standing orders) have no Fleet route from this page yet.
          return;
      }
    },
    [
      raises,
      sessions,
      kept.open,
      readTalk,
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
