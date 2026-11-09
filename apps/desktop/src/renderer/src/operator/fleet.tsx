// The Operator drawn from Fleet: the board it serves (what it is on, what waits for it, what it settled), every live Job's
// Needs you items, its talk, and its settings. A press goes to Fleet's route for it; what this window keeps is only what
// is open in Discussion, where an item came up there, and what he pressed here since it opened.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type {
  OperatorBoard,
  OperatorBoardItem,
  OperatorBoardKind,
  OperatorBoardSettled,
  OperatorBoardSuggestion,
  OperatorTalk,
  Outcome,
  RepositorySummary,
  WaitingItem,
} from "@armada/protocol";
import { jobIdOf } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../shared/bridge";
import { useSessionsEverywhere } from "../sessions-draft";
import { stateOf } from "../state-feed";
import { saveSetting, useSetting } from "../use-setting";
import type {
  OperatorDid,
  OperatorLogRow,
  OperatorMessage,
  OperatorPosition,
  OperatorRaise,
  OperatorRole,
  OperatorTurn,
} from "./model";
import { OperatorHeld } from "./store";
import type { Act, OperatorState } from "./store";
import { openJobWalk } from "../job-walk/open";
import { openJob } from "./takes/board";

const KIND_TAG: Readonly<Record<OperatorBoardKind, string>> = {
  start: "New Job",
  question: "Question",
  card: "Question",
  stall: "Quiet Job",
  report: "Report",
};

/** What kind of ask a Needs you card is, the tag above the Job's name. */
const SOURCE_TAG: Partial<Readonly<Record<WaitingItem["source"], string>>> = {
  operator_suggestion: "Suggestion",
  agent: "Question",
  ask_card: "Question",
  window_ask: "Question",
  walk: "Walk",
  draft: "Draft",
  plan: "Plan",
  sketch: "Sketch",
  document: "Page",
  permission: "Permission",
  done: "Finished",
  replied: "Reply",
  first_message: "First message",
  delivery: "Landing",
  slot_choice: "Slot",
};

const OPEN_JOB = "Open Job";
/** A walk the Job shows: its Job opens with the walk's tab in front. */
const OPEN_WALK = "Open walk";

/** What pressing Discuss asks the Operator about the item. */
const OPENING_ASK = "What is this, and what do you recommend?";

/** The id Fleet knows an item by: a Job's item is `<job> <item>`, and Fleet reads the item's own id. */
const aboutOf = (raise: string) => raise.split(" ").slice(1).join(" ") || raise;

const ROLE_KEY: Readonly<Record<OperatorRole, string>> = {
  dispatch: "features.operatorDispatch",
  delegate: "features.operatorDelegate",
  answer: "features.operatorAnswer",
  file: "features.operatorFile",
};

/** A board item as a card: the Job it is about by its name, where Fleet names one, and its words under it. */
const turnOf = (item: OperatorBoardItem, sessions: readonly Session[]): OperatorTurn => ({
  id: item.id,
  text: item.title,
  tag: KIND_TAG[item.kind],
  ...(item.session_id === undefined
    ? {}
    : {
        job: {
          session: item.session_id,
          title: sessions.find((each) => each.id === item.session_id)?.title ?? item.title,
        },
      }),
  ...(item.repository === undefined
    ? {}
    : {
        repository: item.repository,
      }),
});

/** The Log's column for the kind an outcome is written with, `Kind: what`. */
const DID: Readonly<Record<string, OperatorDid>> = {
  Answered: "answered",
  Raised: "raised",
  Started: "started",
  "Handed over": "existing",
  Sent: "existing",
  Merged: "merged",
  "Kept in touch": "in_touch",
  Filed: "filed",
  Linked: "filed",
  "Passed on": "passed_on",
  Skipped: "passed_on",
};

/** What a settled item was, as the Log's sentence names it. */
const ABOUT: Readonly<Record<OperatorBoardSettled["kind"], NonNullable<OperatorLogRow["about"]>>> = {
  start: "start",
  question: "question",
  card: "question",
  stall: "stall",
  report: "report",
};

/** One of the Log's rows that settled something, as the board's Done serves it. */
function logOf(one: OperatorBoardSettled, sessions: readonly Session[]): OperatorLogRow {
  const at = one.outcome.indexOf(": ");
  const kind = at === -1 ? one.outcome : one.outcome.slice(0, at);
  const session = sessions.find((each) => each.id === one.session_id);
  return {
    id: one.id,
    at: one.at,
    from:
      one.session_id === undefined
        ? {
            kind: "report",
            by: "you",
          }
        : {
            kind: "job",
            job: {
              session: one.session_id,
              title: session?.title ?? one.title,
            },
          },
    item: one.title,
    did: DID[kind] ?? "answered",
    why: one.because ?? "",
    by: one.by === "person" ? "you" : "operator",
    ...(at === -1
      ? {}
      : {
          what: one.outcome.slice(at + 2),
        }),
    about: ABOUT[one.kind],
    ...(one.repository === undefined
      ? {}
      : {
          repository: one.repository,
        }),
  };
}

/** One Needs you item of a Job, as a card: the Operator's pick first where it read one. */
function raiseOf(session: Session, item: WaitingItem): OperatorRaise {
  const labels = (item.options ?? []).map((one) => one.label);
  const pick = item.operator_read?.pick;
  const ways =
    item.source === "walk" && item.act?.target !== undefined
      ? [
          OPEN_WALK,
          OPEN_JOB,
        ]
      : labels.length === 0
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
  const recommends =
    pick !== undefined && labels.includes(pick)
      ? pick
      : labels.find((one) => /\(recommended\)/i.test(one));
  const branch = session.attachments.find((one) => one.kind === "branch");
  return {
    id: `${session.id} ${item.id}`,
    suggests,
    ...(recommends === undefined
      ? {}
      : {
          recommends,
        }),
    ...(branch?.kind === "branch"
      ? {
          branch: branch.name,
        }
      : {}),
    repository: session.manifestId ?? "",
    ...(session.manifestId === undefined
      ? {}
      : {
          manifest: session.manifestId,
        }),
    // The Job's name is the title; the line under it is what it needs from him.
    item: title,
    read:
      item.source === "done"
        ? (session.done?.summary ?? "Says it's finished. Look it over and end it")
        : item.text,
    ways,
    does: "raised",
    tag: SOURCE_TAG[item.source] ?? "Ask",
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
  "decide",
  "suggest",
  "read_only",
];
/** A saved level, its old words read as their new ones; anything else is the lowest. */
const positionOf = (value: unknown): OperatorPosition =>
  value === "on"
    ? "decide"
    : POSITIONS.includes(value as OperatorPosition)
      ? (value as OperatorPosition)
      : "read_only";

/** A refusal in words, for Discussion. */
function refusedSaid(outcome: Outcome): string {
  if (outcome.ok) return "";
  if (outcome.why === "not_connected") return "Fleet is not connected.";
  if (outcome.why !== "refused") return "Fleet did not answer.";
  const { code, message } = outcome.error;
  if (code === "fleet.operator_off")
    return "I'm switched off, so I can't answer. Switch me on in the Permissions tab.";
  if (code === "fleet.operator_ceiling" || code === "fleet.operator_paused")
    return "I'm paused: your week's plan is past the share set in Permissions.";
  if (code === "fleet.operator_said_nothing") return "I had nothing to say to that.";
  return message;
}

/** What a Job started from Discussion is told: the item, its Job, and what to bring back. */
function briefOf(raise: OperatorRaise, what: "investigate" | "review"): string {
  const job = `the Job "${raise.job?.title}" (${raise.job?.session})`;
  const asks = `It waits on the person for this: ${raise.read}`;
  return what === "review"
    ? `Review the work of ${job}${raise.branch === undefined ? "" : ` on branch ${raise.branch}`}. ${asks}\n\nRead its diff against main and change nothing. Report what is wrong or missing, with file:line, and whether its answer to the person should change.`
    : `Investigate for ${job}. ${asks}\n\nRead the code and change nothing. Answer the question with the options there are, what each costs, and which you would pick, with file:line.`;
}

type Kept = {
  /** The Needs you card open in Discussion. */
  open?: string;
  /** The card as it was when it opened, so it stays pinned when its Job settles it elsewhere. */
  discussing?: OperatorRaise;
  /** Talk before this time was cleared from Discussion. Fleet keeps it, for the Operator to read. */
  cleared?: string;
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
  const [repositories, setRepositories] = useState<readonly RepositorySummary[]>([]);
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
  const pause = useSetting("limits.operatorPlanPause");
  const model = useSetting("models.operator");

  useEffect(() => {
    const source = stateOf(window.armada);
    let on = true;
    const take = (state: BridgeState) => {
      if (!on) return;
      setBoard(state.operatorBoard ?? null);
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

  const raises = useMemo(() => {
    const waiting = sessions
      .filter(live)
      .flatMap((session) =>
        (session.waitingFor ?? []).map((item) => ({
          session,
          item,
        })),
      )
      // Oldest first, so a card keeps its place while its Job's record changes around it.
      .sort((a, b) => a.item.since.localeCompare(b.item.since))
      .map(({ session, item }) => raiseOf(session, item));
    const settledIds = new Set(kept.settled.map((one) => one.id));
    const notes = (board?.suggestions ?? [])
      .filter((one) => !kept.pressing.includes(one.id))
      .map(noteOf);
    const held = [
      ...waiting.filter((one) => !settledIds.has(one.id)),
      ...notes,
      ...kept.settled,
    ];
    // The card he is discussing stays, saying it was settled, when its Job takes it off its list.
    const gone =
      kept.discussing !== undefined && !held.some((one) => one.id === kept.discussing?.id)
        ? [
            {
              ...kept.discussing,
              settled: "Settled in its Job",
              settledBy: "In its Job",
            },
          ]
        : [];
    return [
      ...held,
      ...gone,
    ];
  }, [
    sessions,
    board,
    kept.settled,
    kept.pressing,
    kept.discussing,
  ]);

  const state = useMemo((): OperatorState => {
    const turns: OperatorTurn[] = [
      ...(board?.now === undefined
        ? []
        : [
            turnOf(board.now, sessions),
          ]),
      ...(board?.next ?? []).map((one) => turnOf(one, sessions)),
      ...(board?.done ?? []).map((one) => ({
        ...turnOf(one, sessions),
        done: one.outcome,
        by: one.by === "person" ? "You" : "Operator",
        at: one.at,
        ...(one.because === undefined || one.because === ""
          ? {}
          : {
              because: one.because,
            }),
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
    ]
      .filter((one) => kept.cleared === undefined || (one.at ?? "") > kept.cleared)
      .sort((a, b) => (a.at ?? "").localeCompare(b.at ?? ""));
    return {
      // The setting, since Fleet publishes no board change when the switch moves.
      on: typeof switched === "boolean" ? switched : (board?.on ?? false),
      positions: {
        dispatch: positionOf(dispatch),
        delegate: positionOf(delegate),
        answer: positionOf(answer),
        file: positionOf(file),
      },
      pause: Number(pause ?? 80),
      ...(board?.paused === undefined
        ? {}
        : {
            paused: board.paused,
          }),
      ...(typeof model === "string"
        ? {
            model,
          }
        : {}),
      log: (board?.done ?? []).map((one) => logOf(one, sessions)),
      switched: board?.switched ?? [],
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
    sessions,
    switched,
    dispatch,
    delegate,
    answer,
    file,
    pause,
    model,
  ]);

  /** His words to the Operator, about an item where one is open: drawn at once, and answered or refused in the talk. */
  const send = useCallback(
    (words: string, about: string | undefined) => {
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
              at: new Date().toISOString(),
            },
          ],
        }));
      });
    },
    [
      readTalk,
    ],
  );

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
        case "discuss": {
          const raise = raises.find((each) => each.id === one.raise);
          if (raise === undefined) return;
          setKept((was) => ({
            ...was,
            open: one.raise,
            discussing: raise,
            marks: [
              ...was.marks,
              mark(one.raise),
            ],
          }));
          // Pressing Discuss is asking this, so it is his words in the talk and the Operator answers it at once.
          send(OPENING_ASK, aboutOf(one.raise));
          return;
        }
        case "not-now":
          setKept((was) => {
            const { open: _open, discussing: _discussing, ...rest } = was;
            return rest;
          });
          return;
        case "clear":
          setKept((was) => ({
            ...was,
            cleared: now(),
            marks: [],
            said: [],
          }));
          return;
        case "dispatch": {
          const raise = raises.find((each) => each.id === one.raise);
          if (raise?.job === undefined) return;
          const title = `${one.what === "review" ? "Review" : "Investigate"}: ${raise.job.title}`;
          const root = repositories.find((each) => jobIdOf(each) === raise.manifest)?.root;
          const said = (text: string, opens?: OperatorMessage["opens"]) =>
            setKept((was) => ({
              ...was,
              said: [
                ...was.said,
                {
                  id: fresh("started"),
                  from: "operator",
                  text,
                  at: now(),
                  ...(opens === undefined
                    ? {}
                    : {
                        opens,
                      }),
                },
              ],
            }));
          if (root === undefined)
            return said(`${title} did not start: its repository is not served here.`);
          void window.armada.startSession(title, root, briefOf(raise, one.what)).then((started) =>
            started.ok
              ? said(`Started Job ${title}. Its answer is in its own thread.`, {
                  session: started.value.id,
                  title,
                })
              : said(`${title} did not start: ${refusedSaid(started.outcome)}`),
          );
          return;
        }
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
          if (one.way === OPEN_WALK) {
            const walk = sessions
              .find((each) => each.id === session)
              ?.waitingFor?.find((each) => each.id === item)?.act?.target;
            return walk === undefined
              ? openJob(raise.job.session)
              : openJobWalk(raise.job.session, walk);
          }
          const choice = sessions
            .find((each) => each.id === session)
            ?.waitingFor?.find((each) => each.id === item)
            ?.options?.findIndex((each) => each.label === one.way);
          if (choice === undefined || choice === -1) return openJob(raise.job.session);
          const outcome = `${raise.suggests ? "Done" : "Answered"}: ${one.way}`;
          setKept((was) => {
            const { open, discussing, ...rest } = was;
            return {
              ...rest,
              ...(open === undefined || open === one.raise
                ? {}
                : {
                    open,
                  }),
              ...(discussing === undefined || discussing.id === one.raise
                ? {}
                : {
                    discussing,
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
                  settledBy: "You",
                  settledAt: now(),
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
          send(words, kept.open === undefined ? undefined : aboutOf(kept.open));
          return;
        }
        case "on":
          void saveSetting("features.operator", one.on);
          return;
        case "position":
          void saveSetting(ROLE_KEY[one.role], one.position);
          return;
        case "pause":
          void saveSetting("limits.operatorPlanPause", one.share);
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
