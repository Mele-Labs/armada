// A Job: its whole thread, read from the Mac that holds it, with every reply the phone makes sent to
// that Mac. What it waits on sits at the end of the thread; a message box sits under it. The frame, the
// thread and the box are the ones Bridge's Job view draws. A command to run and a draft to settle are cards at the
// thread's end too; the header carries the context figure, the Jobs this one is linked to, and the Job's other acts.

import { Fragment, useEffect, useState, type ReactNode } from "react";
import {
  Check,
  ChevronRight,
  CirclePause,
  CornerDownRight,
  FileText,
  Files,
  FlaskConical,
  GitFork,
  Link as Linked,
  ListTodo,
  MessageCircleQuestion,
  Play,
  PowerOff,
  RefreshCw,
  Scale,
  ScrollText,
  Shapes,
  Square,
  SquareTerminal,
  type LucideIcon,
} from "lucide-react";
import { AskCards, SessionThread } from "./kit/components/compositions/SessionThread/SessionThread";
import { Badge } from "./kit/components/primitives/Badge/Badge";
import { Button } from "./kit/components/primitives/Button/Button";
import { Card } from "./kit/components/primitives/Card/Card";
import { Select } from "./kit/components/primitives/Select/Select";
import { SessionComposer } from "./kit/components/compositions/SessionComposer/SessionComposer";
import { SessionFrame } from "./kit/components/compositions/SessionFrame/SessionFrame";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import type { DropdownMenuEntry } from "./kit/components/primitives/DropdownMenu/DropdownMenu";
import { tokensSaid } from "./kit/components/compositions/SessionFrame/ContextFigure";
import type { CallAnswer, QuestionAnswer, StartCondition, WaitingItem } from "./wire";
import type { Session, SessionWait, StartWhen } from "./kit/screens/draft/sessions";
import {
  START_WHEN,
  conditionOfWhen,
  planPartsOf,
  startsFrom,
  startsWhenSaid,
} from "./kit/screens/session-waits";
import { jobStateOf } from "./kit/screens/session-state";
import { lapsedAnswerText } from "./kit/screens/sessions-wire";
import { documentAnswerOf, sketchAnswerOf } from "./kit/screens/thread-cards";

import { getJson } from "./client";
import { holderOf, macOf, start, useFleet } from "./fleet";
import type { ModelsBody } from "./gateway";
import { refreshPreview } from "./refresh";
import { actsOn, attempt, attemptPage, readShownMedia, useJob, type Acts } from "./jobState";
import { WithKeptStyles } from "./keptStyles";
import type { Machine } from "./machines";
import { ArtifactCard } from "./Artifact";
import { OnMac, PressCard, RunCard, WaitingAsk } from "./cards";
import { BandCard, Well } from "./kit/components/compositions/BandCard/BandCard";
import { DraftText } from "./kit/components/compositions/SessionThread/DraftRow";
import { PageScreen } from "./PageBody";
import { asks, ofParts, opensAsk, partsOf, useArtifacts } from "./artifacts";
import { TryLine, useTryCards } from "./ReadyToTry";
import {
  artifactPath,
  artifactsPath,
  judgePath,
  documentPath,
  draftPath,
  filePath,
  go,
  jobPath,
  sketchPath,
} from "./router";
import {
  approvalOf,
  askedOf,
  asksOf,
  inAskOrder,
  choicesOf,
  documentOf,
  draftsOf,
  handedOf,
  linksOf,
  onMacOf,
  pressesOf,
  rowsOf,
  runsOf,
  sessionOfThread,
  stepsOf,
  takesWords,
  withArtifacts,
  withEarlier,
  type AskKind,
  type JobLink,
  type OpenDraft,
} from "./thread";
import { Back, Band, Figure, Trouble } from "./ui";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** Runs a reply and keeps its refusal to show. */
function useReply(machine: Machine): [
  string,
  (act: () => Promise<void>, page?: () => void) => Promise<boolean>,
] {
  const [says, setSays] = useState("");
  // `page` is given for an answer to a sketch or a document: what to read again where the page changed under him.
  const run = async (act: () => Promise<void>, page?: () => void) => {
    setSays("");
    const refused =
      page === undefined ? await attempt(act, machine) : await attemptPage(act, machine, page);
    if (refused !== null) setSays(refused);
    return refused === null;
  };
  return [
    says,
    run,
  ];
}

function PlanChanges({ keep, send }: { keep: string; send: (text: string) => Promise<boolean> }) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label="Changes to the plan">
      <Textarea
        keep={keep}
        rows={2}
        aria-label="Changes to the plan"
        placeholder="Changes to the plan"
        value={words}
        onChange={(event) => setWords(event.target.value)}
      />
      <Button
        variant="secondary"
        disabled={words.trim() === ""}
        onClick={() => void send(words.trim()).then((went) => went && setWords(""))}
      >
        Send changes
      </Button>
    </div>
  );
}

/** An ask at the thread's foot: the item that stands for it on Fleet's list, its kind, and its card as Pocket draws it. */
type AskAtFoot = {
  item: WaitingItem;
  kind: AskKind;
  /** `titled` where the first card holds the question already. */
  card: (titled: boolean) => ReactNode;
};

/** Each kind's glyph, Bridge's own. */
const KIND_GLYPH: Record<AskKind, LucideIcon> = {
  takes: Scale,
  question: MessageCircleQuestion,
  run: MessageCircleQuestion,
  build: FlaskConical,
  page: ScrollText,
  plan: ListTodo,
  drawing: Shapes,
  draft: FileText,
};

/**
 * The Job's first open ask as one card: the strip of steps, the question and why it is first, then the ask's own card.
 * The rest wait under See all, in the same order, each drawn as it is on its own.
 */
function FirstAsk({ asks }: { asks: readonly AskAtFoot[] }) {
  const [all, setAll] = useState(false);
  const first = asks[0];
  if (first === undefined) return null;
  const { now, next, more } = stepsOf(asks);
  const why = first.item.why?.trim() ?? "";
  return (
    <>
      <Card className="pk-first" role="article" aria-label={first.item.text}>
        <ol className="pk-first__steps" aria-label="Steps">
          {now === undefined ? null : (
            <li aria-label={`Now: ${now.words}`}>
              <Badge status="awaiting-review" icon={KIND_GLYPH[now.kind]}>
                {`1 ${now.words}`}
              </Badge>
            </li>
          )}
          {next === undefined ? null : (
            <li className="pk-first__step" aria-label={`Next: ${next.words}`}>
              <ChevronRight size={12} strokeWidth={2} aria-hidden="true" />
              {`2 ${next.words}`}
            </li>
          )}
          {more === 0 ? null : <li className="pk-first__step">{`· ${String(more)} more`}</li>}
        </ol>
        <div className="pk-first__head">
          <h3 className="pk-question">{first.item.text}</h3>
          {why === "" ? null : <p className="pk-first__why">{why}</p>}
        </div>
        {first.card(true)}
      </Card>
      {asks.length < 2 ? null : (
        <Button variant="ghost" aria-expanded={all} onClick={() => setAll((was) => !was)}>
          {all ? "Show only the first" : `See all ${String(asks.length)} asks`}
        </Button>
      )}
      {all
        ? asks.slice(1).map((one) => <Fragment key={one.item.id}>{one.card(false)}</Fragment>)
        : null}
    </>
  );
}

/** A draft the agent proposed with `propose_draft`, as the Band card draws it: approved as written, or sent back with a note. */
function DraftCard({
  draft,
  keep,
  approve,
  sendBack,
  dismiss,
}: {
  draft: OpenDraft;
  keep: string;
  approve: () => Promise<boolean>;
  sendBack: (note: string) => Promise<boolean>;
  dismiss: () => Promise<boolean>;
}) {
  return (
    <BandCard
      as="div"
      facts={{
        form: "draft",
        title: draft.title,
        stand: "asking",
      }}
      keep={keep}
      well={
        <Well>
          <DraftText text={draft.text} />
        </Well>
      }
      acts={{
        approve: () => void approve(),
        ask: (note) => void sendBack(note),
        ...(draft.proposed === true
          ? {
              dismiss: () => void dismiss(),
            }
          : {}),
      }}
      asked
    />
  );
}

/** The header's second line: the Job this one started from, the Jobs it started, each a chip opening that Job; then the context figure. */
function HeadFacts({
  from,
  started,
  context,
  open,
}: {
  from?: JobLink;
  started: readonly JobLink[];
  context?: {
    tokens: number;
    window: number;
  };
  open: (id: string) => void;
}) {
  const chip = (one: JobLink, Glyph: typeof Linked, name: string) => (
    <Tooltip key={one.id} label={`${name}: ${one.title}`}>
      <Button
        variant="ghost"
        size="sm"
        aria-label={`${name}: ${one.title}`}
        onClick={() => open(one.id)}
      >
        <Glyph size={12} strokeWidth={2} aria-hidden="true" />
        <span className="pk-head__chip-title">{one.title}</span>
      </Button>
    </Tooltip>
  );
  const fill =
    context === undefined || context.window <= 0
      ? undefined
      : `${tokensSaid(context.tokens)} / ${tokensSaid(context.window)}`;
  if (from === undefined && started.length === 0 && fill === undefined) return null;
  return (
    <span className="pk-head__facts">
      <span className="pk-head__links">
        {from === undefined ? null : chip(from, Linked, "Started from")}
        {started.map((one) => chip(one, CornerDownRight, "Started"))}
      </span>
      {fill === undefined ? null : (
        <Figure name="Context" loose>
          {fill}
        </Figure>
      )}
    </span>
  );
}

const EFFORTS: readonly string[] = [
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
];

/** The models this Mac's Fleet offers, read once per Job opened. */
function useModels(machine: Machine): ModelsBody | undefined {
  const [models, setModels] = useState<ModelsBody | undefined>(undefined);
  useEffect(() => {
    let gone = false;
    getJson<ModelsBody>(machine, "/api/models").then(
      (read) => !gone && setModels(read),
      () => undefined,
    );
    return () => {
      gone = true;
    };
  }, [
    machine.origin,
  ]);
  return models;
}

/** Refresh preview, on every Job's menu: it moves the Mac onto the preview, and not the Job. */
const REFRESH_ENTRY = {
  kind: "item",
  id: "refresh",
  label: "Refresh preview",
  icon: RefreshCw,
} as const;

/** The entry that opens the Job's artifacts, which a Job that showed none does not carry. */
const JUDGE_ENTRY = {
  kind: "item",
  id: "judge",
  label: "Judge…",
  icon: Scale,
} as const;

const ARTIFACTS_ENTRY = {
  kind: "item",
  id: "artifacts",
  label: "Artifacts",
  icon: Files,
} as const;

/** The header's ⋯ menu: Stop while a turn runs, Pause or Resume, Fork, Judge…, Artifacts where the Job showed any, the model and effort, and End. An ended Job keeps Fork, Judge… and Artifacts, and a waiting one, which runs nothing, End and Artifacts. */
function menuOf(
  session: Session,
  models: ModelsBody | undefined,
  showed: boolean,
): DropdownMenuEntry[] {
  if (session.waits !== undefined)
    return [
      ...(showed
        ? [
            ARTIFACTS_ENTRY,
          ]
        : []),
      REFRESH_ENTRY,
      {
        kind: "item",
        id: "end",
        label: "End job",
        icon: PowerOff,
        danger: true,
      },
    ];
  if (session.dead !== undefined)
    return [
      {
        kind: "item",
        id: "fork",
        label: "Fork",
        icon: GitFork,
      },
      JUDGE_ENTRY,
      ...(showed
        ? [
            ARTIFACTS_ENTRY,
          ]
        : []),
      REFRESH_ENTRY,
    ];
  const model = session.model ?? models?.session;
  const efforts = models?.offered?.find((one) => one.name === model)?.efforts ?? EFFORTS;
  return [
    ...(session.turn.state === "working"
      ? [
          {
            kind: "item",
            id: "stop",
            label: "Stop",
            icon: Square,
          } as const,
        ]
      : []),
    session.paused === true
      ? ({
          kind: "item",
          id: "resume",
          label: "Resume",
          icon: Play,
        } as const)
      : ({
          kind: "item",
          id: "pause",
          label: "Pause",
          icon: CirclePause,
        } as const),
    {
      kind: "item",
      id: "fork",
      label: "Fork",
      icon: GitFork,
    },
    JUDGE_ENTRY,
    ...(showed
      ? [
          ARTIFACTS_ENTRY,
        ]
      : []),
    REFRESH_ENTRY,
    ...(models === undefined || models.models.length === 0
      ? []
      : [
          {
            kind: "separator",
            id: "model-rule",
          } as const,
          {
            kind: "label",
            id: "model-label",
            label: "Model",
          } as const,
          ...models.models.map(
            (name): DropdownMenuEntry => ({
              kind: "item",
              id: `model:${name}`,
              label: name,
              selected: name === model,
            }),
          ),
        ]),
    {
      kind: "separator",
      id: "effort-rule",
    },
    {
      kind: "label",
      id: "effort-label",
      label: "Effort",
    },
    ...efforts.map(
      (name): DropdownMenuEntry => ({
        kind: "item",
        id: `effort:${name}`,
        label: name,
        selected: name === session.effort,
      }),
    ),
    {
      kind: "separator",
      id: "end-rule",
    },
    {
      kind: "item",
      id: "end",
      label: "End job",
      icon: PowerOff,
      danger: true,
    },
  ];
}

/** What a waiting Job holds, as Bridge's waiting pane reads it: its brief, the Job it waits on, and when it starts and from where. */
function WaitingFacts({
  machine,
  wait,
  open,
  change,
}: {
  machine: Machine;
  wait: SessionWait;
  open: (id: string) => void;
  change: (condition: StartCondition) => void;
}) {
  const [waitedOn] = useJob(machine, wait.on.id);
  const parts = planPartsOf(
    waitedOn.thread === undefined ? undefined : sessionOfThread(waitedOn.thread),
  );
  const set = (when: StartWhen, part: string | undefined) =>
    change(
      conditionOfWhen(
        when,
        part === undefined
          ? undefined
          : {
              id: part,
            },
        parts,
      ),
    );
  return (
    <dl className="pk-facts">
      <div className="pk-facts__row">
        <dt>Brief</dt>
        <dd className="pk-facts__brief">{wait.brief}</dd>
      </div>
      <div className="pk-facts__row">
        <dt>Waits on</dt>
        <dd>
          <button type="button" className="pk-facts__link" onClick={() => open(wait.on.id)}>
            <SquareTerminal size={14} aria-hidden="true" />
            <span>{wait.on.title}</span>
          </button>
        </dd>
      </div>
      <div className="pk-facts__row">
        <dt>Starts when</dt>
        <dd className="pk-facts__when">
          <Select
            aria-label="Starts when"
            value={wait.when}
            onChange={(event) => {
              const when = event.target.value as StartWhen;
              set(when, when === "plan" ? parts[0]?.id : undefined);
            }}
          >
            {START_WHEN.map((one) => (
              <option
                key={one.id}
                value={one.id}
                disabled={one.id === "plan" && parts.length === 0}
              >
                {one.label}
              </option>
            ))}
          </Select>
          {wait.when !== "plan" ? null : (
            <Select
              aria-label="Plan part"
              value={wait.part?.id ?? parts[0]?.id ?? ""}
              onChange={(event) => set("plan", event.target.value)}
            >
              {parts.length === 0 && wait.part !== undefined ? (
                <option value={wait.part.id}>{wait.part.title}</option>
              ) : null}
              {parts.map((one) => (
                <option key={one.id} value={one.id}>
                  {`${one.kind === "group" ? "Group" : "Task"}: ${one.title}`}
                </option>
              ))}
            </Select>
          )}
        </dd>
      </div>
      {wait.needs === "" ? null : (
        <div className="pk-facts__row">
          <dt>Needs</dt>
          <dd>{wait.needs}</dd>
        </div>
      )}
      <div className="pk-facts__row">
        <dt>Starts from</dt>
        <dd data-mono>{startsFrom(wait)}</dd>
      </div>
    </dl>
  );
}

function Thread({ machine, id, several }: { machine: Machine; id: string; several: boolean }) {
  const [read, reread] = useJob(machine, id);
  const [says, run] = useReply(machine);
  const [skipped, setSkipped] = useState<string | undefined>(undefined);
  const acts: Acts = actsOn(machine, reread);
  const models = useModels(machine);
  const home = () => go("/");
  const held = read.thread?.session.hosted?.start !== undefined;
  // A Job still on its start card opens as a draft.
  useEffect(() => {
    if (held) go(draftPath(machine.origin, id), true);
  }, [
    held,
    machine.origin,
    id,
  ]);
  // A Job handed on opens as the Job that took its place, whose thread carries its turns.
  const later = handedOf(read.thread?.session, "replaced_by");
  useEffect(() => {
    if (later !== undefined) go(jobPath(machine.origin, later), true);
  }, [
    later,
    machine.origin,
  ]);
  const earlierId = handedOf(read.thread?.session, "replaces");
  // The Job it took the place of stands above its first row, so it is read once every row of this one is held.
  const [earlier] = useJob(
    earlierId === undefined || read.older !== false ? undefined : machine,
    earlierId ?? "",
  );
  // The artifacts are read only for a Job that showed one, and the thread's cards only while one waits on him.
  const showed = (read.thread?.session.attachments ?? []).some(
    (one) =>
      one.kind === "artifact" &&
      [
        "sketch",
        "document",
        "draft",
        "page",
      ].includes(one.detail?.["form"] ?? ""),
  );
  const [artifacts] = useArtifacts(showed ? machine : undefined, id);
  const trying = (read.thread?.session.waiting_for ?? []).some(
    (one) => one.source === "ready_to_try",
  );
  const [tryCards] = useTryCards(machine, id, trying);
  if (read.thread === undefined) {
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={home} back="Home" />
        <div className="pk-body">
          <Trouble says={read.error ?? ""} />
        </div>
      </section>
    );
  }
  const session = sessionOfThread(read.thread);
  const keep = (what: string) => `pocket:${machine.origin}:${id}:${what}`;
  const asked = askedOf(session);
  const call = session.asked?.call;
  const plan = session.work?.plan;
  const approval = approvalOf(session, () => {
    if (plan?.version !== undefined)
      void run(() =>
        acts.approvePlan({
          session_id: id,
          version: plan.version!,
        }),
      );
  });
  const onAnswer = (answer?: string, answers?: QuestionAnswer[], folder?: string) =>
    call === undefined
      ? undefined
      : run(() =>
          acts.answerAsk({
            session_id: id,
            call,
            answer: (answer ?? "allow_once") as CallAnswer,
            ...(answers === undefined
              ? {}
              : {
                  answers,
                }),
            ...(folder === undefined
              ? {}
              : {
                  folder,
                }),
          }),
        );
  const lapsed =
    session.lapsed === undefined || session.lapsed.id === skipped
      ? undefined
      : {
          id: session.lapsed.id,
          questions: session.lapsed.questions,
          onAnswer: (answers: QuestionAnswer[]) =>
            void run(() =>
              acts.message({
                session_id: id,
                text: lapsedAnswerText(answers),
              }),
            ),
          onSkip: () => setSkipped(session.lapsed?.id),
        };
  const runs = runsOf(session, read.thread);
  const drafts = draftsOf(session);
  const presses = pressesOf(session);
  const onMac = onMacOf(session);
  const links = linksOf(read.thread);
  const openJob = (other: string) => go(jobPath(machine.origin, other));
  const onMenu = (entry: string) => {
    const job = {
      session_id: id,
    };
    if (entry === "stop") void run(() => acts.stop(job));
    else if (entry === "pause") void run(() => acts.pause(job));
    else if (entry === "resume") void run(() => acts.resume(job));
    else if (entry === "end") void run(() => acts.close(job));
    else if (entry === "fork") void run(async () => openJob((await acts.fork(job)).id));
    else if (entry === "artifacts") go(artifactsPath(machine.origin, id));
    else if (entry === "judge") go(judgePath(machine.origin, id));
    else if (entry === "refresh") void run(() => refreshPreview(machine));
    else if (entry.startsWith("model:") || entry.startsWith("effort:")) {
      const model = entry.startsWith("model:") ? entry.slice("model:".length) : session.model;
      const effort = entry.startsWith("effort:") ? entry.slice("effort:".length) : session.effort;
      void run(() =>
        acts.tune({
          session_id: id,
          ...(model === undefined
            ? {}
            : {
                model,
              }),
          ...(effort === undefined
            ? {}
            : {
                effort,
              }),
          mode: session.mode ?? "auto",
        }),
      );
    }
  };
  const doneSince =
    read.thread.session.state === "live" ? read.thread.session.done_since : undefined;
  const doneSummary = doneSince === undefined ? undefined : read.thread.session.done_summary;
  const planPending =
    approval !== undefined &&
    !approval.approved &&
    plan?.version !== undefined &&
    approval.groups[0] !== undefined;
  const openArtifact = (artifact: string) => go(artifactPath(machine.origin, id, artifact));
  const waiting = session.dead === undefined ? (session.waitingFor ?? []) : [];
  // Each card at the thread's foot that a waiting item stands for is one ask, in the Job's order. A card no item names yet
  // (an older Mac's, or one read before Fleet's list) keeps its place above them.
  const foot: AskAtFoot[] = [];
  const loose: ReactNode[] = [];
  const openTries = (tryCards ?? []).filter((card) => card.state === "open");
  const tryItem = waiting.find((one) => one.source === "ready_to_try");
  const tryLines = openTries.map((card) => (
    <TryLine key={card.id} machine={machine} id={id} card={card} />
  ));
  if (tryItem === undefined) loose.push(...tryLines);
  else
    foot.push({
      item: tryItem,
      kind: "build",
      card: () => tryLines,
    });
  for (const one of (artifacts.artifacts ?? []).filter((one) => ofParts(one) && asks(one))) {
    const card = <ArtifactCard key={one.id} machine={machine} id={id} artifact={one} />;
    const item = waiting.find(
      (row) => row.id === one.item || row.artifact === one.id || row.id === one.id,
    );
    if (item === undefined) loose.push(card);
    else
      foot.push({
        item,
        kind: partsOf(one).some((part) => part.kind === "takes" && opensAsk(part))
          ? "takes"
          : item.source === "document"
            ? "page"
            : "drawing",
        card: () => card,
      });
  }
  if (asked !== undefined || lapsed !== undefined) {
    const card = (
      <AskCards
        key="asked"
        {...(asked === undefined
          ? {}
          : {
              asked,
              keep: keep(call ?? asked.command),
            })}
        onAnswer={onAnswer}
        {...(lapsed === undefined
          ? {}
          : {
              lapsed,
            })}
      />
    );
    const item =
      call === undefined
        ? undefined
        : waiting.find((one) => one.id === `ask:${call}` || one.id === `perm:${call}`);
    if (item === undefined) loose.push(card);
    else
      foot.push({
        item,
        kind: "question",
        card: () => card,
      });
  }
  for (const item of asksOf(session))
    foot.push({
      item,
      kind: "question",
      card: (titled) => (
        <WaitingAsk
          key={item.id}
          text={item.text}
          choices={choicesOf(item)}
          words={takesWords(item)}
          making={item.creating_since !== undefined}
          refused={item.create_refused ?? ""}
          keep={keep(`waiting:${item.id}`)}
          headed={titled}
          answer={(given) =>
            run(() =>
              acts.answerWaiting({
                session_id: id,
                item_id: item.id,
                ...given,
              }),
            )
          }
        />
      ),
    });
  for (const { item, command, ran } of runs)
    foot.push({
      item,
      kind: "run",
      card: (titled) => (
        <RunCard
          key={item.id}
          text={item.text}
          command={command}
          headed={titled}
          {...(ran === undefined
            ? {}
            : {
                ran,
              })}
          run={() =>
            run(() =>
              acts.run({
                session_id: id,
                command,
              }),
            )
          }
        />
      ),
    });
  for (const draft of drafts) {
    const item = waiting.find((one) => one.id === `draft:${draft.id}`);
    if (item === undefined) continue;
    foot.push({
      item,
      kind: "draft",
      card: () => (
        <DraftCard
          key={draft.id}
          draft={draft}
          keep={keep(`draft:${draft.id}:note`)}
          approve={() =>
            run(() =>
              acts.answerWaiting({
                session_id: id,
                item_id: `draft:${draft.id}`,
                choice: 0,
              }),
            )
          }
          sendBack={(note) =>
            run(() =>
              acts.answerWaiting({
                session_id: id,
                item_id: `draft:${draft.id}`,
                text: note,
              }),
            )
          }
          dismiss={() =>
            run(() =>
              acts.answerWaiting({
                session_id: id,
                item_id: `draft:${draft.id}`,
                choice: 1,
              }),
            )
          }
        />
      ),
    });
  }
  if (planPending) {
    const card = (
      <PlanChanges
        key="plan"
        keep={keep("plan")}
        send={(text) =>
          run(() =>
            acts.planChanges({
              session_id: id,
              version: plan!.version!,
              marks: [
                {
                  kind: "note",
                  id: `phone-${Date.now()}`,
                  group: approval!.groups[0]!.id,
                  text,
                },
              ],
            }),
          )
        }
      />
    );
    const item = waiting.find((one) => one.source === "plan");
    if (item === undefined) loose.push(card);
    else
      foot.push({
        item,
        kind: "plan",
        card: () => card,
      });
  }
  const trailing = (
    <div className="pk-trailing">
      {loose}
      <FirstAsk asks={inAskOrder(foot)} />
      {presses.map(({ item, opens, dismissible }) => (
        <PressCard
          key={item.id}
          text={item.text}
          {...(opens === undefined || opens === id
            ? {}
            : {
                open: () => openJob(opens),
              })}
          {...(dismissible
            ? {
                dismiss: () =>
                  run(() =>
                    acts.dismissWaiting({
                      session_id: id,
                      item_id: item.id,
                    }),
                  ),
              }
            : {})}
        />
      ))}
      {doneSince === undefined ? null : (
        <div className="pk-done" role="status">
          {doneSummary === undefined ? null : <p className="pk-done__says">{doneSummary}</p>}
          <Button
            variant="secondary"
            size="sm"
            onClick={() =>
              void run(() =>
                acts.close({
                  session_id: id,
                }),
              )
            }
          >
            <PowerOff size={14} aria-hidden="true" />
            End job
          </Button>
        </div>
      )}
      {onMac.map((item) => (
        <OnMac key={item.id} text={item.text} />
      ))}
      <Trouble says={says} />
    </div>
  );
  // He is in the Job, so a reply is read.
  const { state, said } = jobStateOf(session, () => true);
  const context = session.context;
  const wait = session.waits;
  // Approved starts the Job when its condition is met; otherwise it opens as a draft then.
  const approved = wait?.held === false;
  const changeWhen = (condition: StartCondition) =>
    void run(() =>
      acts.changeWaiting({
        waiting_session: id,
        condition,
      }),
    );
  return (
    <section className="pk-screen" aria-label={session.title ?? "Job"}>
      <div className="deck-pane pk-pane" data-frame data-bare>
        <SessionFrame
          state={wait === undefined ? state : "after"}
          said={
            several
              ? `${wait === undefined ? said : startsWhenSaid(wait)} · ${machine.name}`
              : wait === undefined
                ? said
                : startsWhenSaid(wait)
          }
          id={id}
          {...(session.title === undefined
            ? {}
            : {
                title: session.title,
              })}
          actions={
            <HeadFacts
              {...(links.from === undefined
                ? {}
                : {
                    from: links.from,
                  })}
              started={links.started}
              {...(context === undefined
                ? {}
                : {
                    context,
                  })}
              open={openJob}
            />
          }
          menu={{
            entries: menuOf(session, models, showed),
            onSelect: onMenu,
          }}
          back={<Back label="Home" onClick={home} />}
        >
          {wait !== undefined ? (
            <div className="armada-session-frame__centre">
              <div className="pk-waiting">
                <WaitingFacts machine={machine} wait={wait} open={openJob} change={changeWhen} />
                {trailing}
              </div>
              <footer className="pk-bar pk-bar--pair pk-bar--tall">
                <Button
                  variant="secondary"
                  aria-pressed={approved}
                  onClick={() =>
                    void run(() =>
                      acts.changeWaiting({
                        waiting_session: id,
                        held: approved,
                      }),
                    )
                  }
                >
                  {approved ? <Check size={14} aria-hidden="true" /> : null}
                  {approved ? "Approved" : "Approve"}
                </Button>
                <Button
                  variant="primary"
                  onClick={() =>
                    void run(() =>
                      acts.startWaiting({
                        session_id: id,
                      }),
                    )
                  }
                >
                  Start now
                </Button>
              </footer>
            </div>
          ) : (
            <div className="armada-session-frame__centre">
              <WithKeptStyles machine={machine}>
                <SessionThread
                  sessionId={`${machine.origin} ${id}`}
                  rows={withArtifacts(
                    withEarlier(rowsOf(session), earlier.thread),
                    artifacts.artifacts,
                    openArtifact,
                  )}
                  {...((read.onOlder ?? earlier.onOlder) === undefined
                    ? {}
                    : {
                        onOlder: (read.onOlder ?? earlier.onOlder)!,
                      })}
                  {...(approval === undefined
                    ? {}
                    : {
                        approval,
                      })}
                  working={session.turn.state === "working"}
                  onAnswer={onAnswer}
                  onOpenSession={openJob}
                  onOpenFile={(path) => go(filePath(machine.origin, id, path))}
                  onReadMedia={(path) => readShownMedia(machine, id, path)}
                  onOpenSketch={(sketch) => go(sketchPath(machine.origin, id, sketch))}
                  onAnswerSketch={(sketch, answer) =>
                    void run(
                      () => acts.answerSketch(sketchAnswerOf(session, sketch, answer)),
                      reread,
                    )
                  }
                  onWithdrawSketch={(sketch) =>
                    void run(() =>
                      acts.dismissWaiting({
                        session_id: id,
                        item_id: sketch,
                      }),
                    )
                  }
                  onWithdrawDocument={(page) =>
                    void run(() =>
                      acts.dismissWaiting({
                        session_id: id,
                        item_id: `document:${page}`,
                      }),
                    )
                  }
                  screenOf={(page, revision) => {
                    const held = documentOf(session, page);
                    return (
                      <PageScreen
                        machine={machine}
                        id={id}
                        path={held?.revisions[revision - 1]}
                        title={held?.title ?? page}
                        keptStyles={held?.keptStyles}
                      />
                    );
                  }}
                  onOpenDocument={(page) => go(documentPath(machine.origin, id, page))}
                  onAnswerDocument={(page, answer) =>
                    void run(
                      () => acts.answerDocument(documentAnswerOf(session, page, answer)),
                      reread,
                    )
                  }
                  trailing={trailing}
                />
              </WithKeptStyles>
              {session.dead !== undefined ? null : (
                <div className="pk-dock">
                  <SessionComposer
                    bare
                    enterBreaks
                    working={session.turn.state === "working"}
                    model={null}
                    effort={null}
                    models={NONE}
                    efforts={NONE}
                    onTune={NOBODY}
                    commands={NONE}
                    taggable={NONE}
                    tags={NONE}
                    onTags={NOBODY}
                    drawn={NONE}
                    onDraw={NOBODY}
                    onRemoveDrawn={NOBODY}
                    draftKey={keep("message")}
                    onSend={(sent) =>
                      run(() =>
                        acts.message({
                          session_id: id,
                          text: sent.text.trim(),
                        }),
                      )
                    }
                  />
                </div>
              )}
            </div>
          )}
        </SessionFrame>
      </div>
    </section>
  );
}

/** Finds the Mac a Job is on, from the path or, for a push that did not say, by asking each Mac. */
export function Job({ origin, id }: { origin: string | undefined; id: string }) {
  const fleet = useFleet();
  const [found, setFound] = useState<string | undefined>(origin);
  const [lost, setLost] = useState(false);
  useEffect(() => {
    if (origin !== undefined) return;
    void holderOf(id).then((mac) => {
      if (mac === undefined) setLost(true);
      else go(jobPath(mac.machine.origin, id), true);
    });
  }, [
    origin,
    id,
  ]);
  useEffect(() => {
    void start();
    setFound(origin);
  }, [
    origin,
  ]);
  const mac = found === undefined ? undefined : macOf(found);
  if (mac === undefined) {
    const says = lost
      ? "No paired Mac holds that Job."
      : fleet.loaded && found !== undefined
        ? "This phone is not paired with that Mac."
        : "";
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={() => go("/")} back="Home" />
        <div className="pk-body">
          <Trouble says={says} />
        </div>
      </section>
    );
  }
  return (
    <Thread
      key={`${mac.machine.origin} ${id}`}
      machine={mac.machine}
      id={id}
      several={fleet.macs.length > 1}
    />
  );
}
