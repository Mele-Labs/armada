// A Job: its whole thread, read from the Mac that holds it, with every reply the phone makes sent to
// that Mac. What it waits on sits at the end of the thread; a message box sits under it. The frame, the
// thread and the box are the ones Bridge's Job view draws. A command to run and a draft to settle are cards at the
// thread's end too; the header carries the context figure, the Jobs this one is linked to, and the Job's other acts.

import { useEffect, useState } from "react";
import { CirclePause, CornerDownRight, GitFork, Link as Linked, Play, PowerOff, Square } from "lucide-react";
import {
  AskCards,
  Button,
  Card,
  CardFooter,
  CardHeader,
  CardTitle,
  Prose,
  ProseCommand,
  ProseRunner,
  SessionComposer,
  SessionFrame,
  SessionThread,
  Textarea,
  Tooltip,
  type DropdownMenuEntry,
  type SessionState,
} from "@armada/components";
import { tokensSaid } from "@armada/components/src/compositions/SessionFrame/ContextFigure";
import type { HelmCallAnswer, QuestionAnswer, WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { lapsedAnswerText } from "@armada/screens/src/sessions-wire";

import { getJson } from "./client";
import { holderOf, macOf, start, useFleet } from "./fleet";
import type { ModelsBody } from "./gateway";
import { actsOn, attempt, useJob, type Acts } from "./jobState";
import type { Machine } from "./machines";
import { draftPath, filePath, go, jobPath, sketchPath } from "./router";
import { approvalOf, askedOf, asksOf, draftsOf, linksOf, rowsOf, runsOf, sessionOfThread, type JobLink, type OpenDraft } from "./thread";
import { Back, Band, Figure, Trouble } from "./ui";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** What a Job is doing, as the one mark in its header. */
const standing = (waiting: boolean, working: boolean, ended: boolean): { state: SessionState; said: string } =>
  ended ? { state: "ended", said: "Ended" } : waiting ? { state: "waiting", said: "Needs you" } : working ? { state: "working", said: "Working" } : { state: "idle", said: "Idle" };

/** Runs a reply and keeps its refusal to show. */
function useReply(machine: Machine): [string, (act: () => Promise<void>) => Promise<boolean>] {
  const [says, setSays] = useState("");
  const run = async (act: () => Promise<void>) => {
    setSays("");
    const refused = await attempt(act, machine);
    if (refused !== null) setSays(refused);
    return refused === null;
  };
  return [says, run];
}

/** Something the agent itself asked: its choices, or the owner's own words. */
function WaitingAsk({ item, keep, answer }: { item: WaitingItem; keep: string; answer: (given: { choice?: number; text?: string }) => Promise<boolean> }) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label={item.text}>
      <p className="pk-question">{item.text}</p>
      {(item.options ?? []).length === 0 ? null : (
        <div className="pk-choices" role="group" aria-label="Options">
          {(item.options ?? []).map((option, choice) => (
            <Button key={option.label} variant="secondary" className="pk-choice" title={option.description} onClick={() => void answer({ choice })}>{option.label}</Button>
          ))}
        </div>
      )}
      <Textarea keep={keep} rows={2} aria-label="Answer" placeholder="Answer" value={words} onChange={(event) => setWords(event.target.value)} />
      <Button variant="primary" disabled={words.trim() === ""} onClick={() => void answer({ text: words.trim() }).then((went) => went && setWords(""))}>Answer</Button>
    </div>
  );
}

function PlanChanges({ keep, send }: { keep: string; send: (text: string) => Promise<boolean> }) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label="Changes to the plan">
      <Textarea keep={keep} rows={2} aria-label="Changes to the plan" placeholder="Changes to the plan" value={words} onChange={(event) => setWords(event.target.value)} />
      <Button variant="secondary" disabled={words.trim() === ""} onClick={() => void send(words.trim()).then((went) => went && setWords(""))}>Send changes</Button>
    </div>
  );
}

/** A command the agent asked the owner to run on the Mac: run there from here, and its exit and tail come back as a `ran` row. */
function RunCard({ item, command, run }: { item: WaitingItem; command: string; run: () => Promise<boolean> }) {
  const [going, setGoing] = useState(false);
  return (
    <Card className="pk-card" role="group" aria-label={item.text}>
      <CardHeader>
        <CardTitle>{item.text}</CardTitle>
      </CardHeader>
      <code className="pk-card__command">{command}</code>
      <CardFooter className="pk-card__acts">
        <Button
          variant="primary"
          pending={going}
          onClick={() => {
            setGoing(true);
            void run().finally(() => setGoing(false));
          }}
        >
          Run
        </Button>
      </CardFooter>
    </Card>
  );
}

/** A draft the agent proposed with `propose_draft`: approved as written, or sent back with a note. */
function DraftCard({ draft, keep, approve, sendBack }: { draft: OpenDraft; keep: string; approve: () => Promise<boolean>; sendBack: (note: string) => Promise<boolean> }) {
  const [noting, setNoting] = useState(false);
  const [note, setNote] = useState("");
  return (
    <Card className="pk-card" role="group" aria-label={draft.title}>
      <CardHeader>
        <CardTitle>{draft.title}</CardTitle>
      </CardHeader>
      <div className="pk-card__text">
        <ProseRunner.Provider value={null}>
          <ProseCommand.Provider value={null}>
            <Prose text={draft.text} />
          </ProseCommand.Provider>
        </ProseRunner.Provider>
      </div>
      {noting ? <Textarea keep={keep} rows={3} autoFocus aria-label="Note" placeholder="Note" value={note} onChange={(event) => setNote(event.target.value)} /> : null}
      <CardFooter className="pk-card__acts">
        {noting ? (
          <>
            <Button variant="ghost" onClick={() => (setNote(""), setNoting(false))}>Cancel</Button>
            <Button variant="primary" disabled={note.trim() === ""} onClick={() => void sendBack(note.trim()).then((went) => went && setNote(""))}>Send back</Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => setNoting(true)}>Send back</Button>
            <Button variant="primary" onClick={() => void approve()}>Approve</Button>
          </>
        )}
      </CardFooter>
    </Card>
  );
}

/** The header's second line: the Job this one started from, the Jobs it started, each a chip opening that Job; then the context figure. */
function HeadFacts({ from, started, context, open }: { from?: JobLink; started: readonly JobLink[]; context?: { tokens: number; window: number }; open: (id: string) => void }) {
  const chip = (one: JobLink, Glyph: typeof Linked, name: string) => (
    <Tooltip key={one.id} label={`${name}: ${one.title}`}>
      <Button variant="ghost" size="sm" aria-label={`${name}: ${one.title}`} onClick={() => open(one.id)}>
        <Glyph size={12} strokeWidth={2} aria-hidden="true" />
        <span className="pk-head__chip-title">{one.title}</span>
      </Button>
    </Tooltip>
  );
  const fill = context === undefined || context.window <= 0 ? undefined : `${tokensSaid(context.tokens)} / ${tokensSaid(context.window)}`;
  if (from === undefined && started.length === 0 && fill === undefined) return null;
  return (
    <span className="pk-head__facts">
      <span className="pk-head__links">
        {from === undefined ? null : chip(from, Linked, "Started from")}
        {started.map((one) => chip(one, CornerDownRight, "Started"))}
      </span>
      {fill === undefined ? null : <Figure name="Context" loose>{fill}</Figure>}
    </span>
  );
}

const EFFORTS: readonly string[] = ["low", "medium", "high", "xhigh", "max"];

/** The models this Mac's Fleet offers, read once per Job opened. */
function useModels(machine: Machine): ModelsBody | undefined {
  const [models, setModels] = useState<ModelsBody | undefined>(undefined);
  useEffect(() => {
    let gone = false;
    getJson<ModelsBody>(machine, "/api/models").then((read) => !gone && setModels(read), () => undefined);
    return () => {
      gone = true;
    };
  }, [machine.origin]);
  return models;
}

/** The header's ⋯ menu: Stop while a turn runs, Pause or Resume, Fork, the model and effort, and End. An ended Job keeps Fork alone. */
function menuOf(session: Session, models: ModelsBody | undefined): DropdownMenuEntry[] {
  if (session.dead !== undefined) return [{ kind: "item", id: "fork", label: "Fork", icon: GitFork }];
  const model = session.model ?? models?.session;
  const efforts = models?.offered?.find((one) => one.name === model)?.efforts ?? EFFORTS;
  return [
    ...(session.turn.state === "working" ? [{ kind: "item", id: "stop", label: "Stop", icon: Square } as const] : []),
    session.paused === true ? ({ kind: "item", id: "resume", label: "Resume", icon: Play } as const) : ({ kind: "item", id: "pause", label: "Pause", icon: CirclePause } as const),
    { kind: "item", id: "fork", label: "Fork", icon: GitFork },
    ...(models === undefined || models.models.length === 0
      ? []
      : [
          { kind: "separator", id: "model-rule" } as const,
          { kind: "label", id: "model-label", label: "Model" } as const,
          ...models.models.map((name): DropdownMenuEntry => ({ kind: "item", id: `model:${name}`, label: name, selected: name === model })),
        ]),
    { kind: "separator", id: "effort-rule" },
    { kind: "label", id: "effort-label", label: "Effort" },
    ...efforts.map((name): DropdownMenuEntry => ({ kind: "item", id: `effort:${name}`, label: name, selected: name === session.effort })),
    { kind: "separator", id: "end-rule" },
    { kind: "item", id: "end", label: "End job", icon: PowerOff, danger: true },
  ];
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
  }, [held, machine.origin, id]);
  if (read.thread === undefined) {
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={home} back="Home" />
        <div className="pk-body"><Trouble says={read.error ?? ""} /></div>
      </section>
    );
  }
  const session = sessionOfThread(read.thread);
  const keep = (what: string) => `pocket:${machine.origin}:${id}:${what}`;
  const asked = askedOf(session);
  const call = session.asked?.call;
  const plan = session.work?.plan;
  const approval = approvalOf(session, () => {
    if (plan?.version !== undefined) void run(() => acts.approvePlan({ session_id: id, version: plan.version! }));
  });
  const onAnswer = (answer?: string, answers?: QuestionAnswer[], folder?: string) =>
    call === undefined
      ? undefined
      : run(() => acts.answerAsk({ session_id: id, call, answer: (answer ?? "allow_once") as HelmCallAnswer, ...(answers === undefined ? {} : { answers }), ...(folder === undefined ? {} : { folder }) }));
  const lapsed =
    session.lapsed === undefined || session.lapsed.id === skipped
      ? undefined
      : {
          id: session.lapsed.id,
          questions: session.lapsed.questions,
          onAnswer: (answers: QuestionAnswer[]) => void run(() => acts.message({ session_id: id, text: lapsedAnswerText(answers) })),
          onSkip: () => setSkipped(session.lapsed?.id),
        };
  const runs = runsOf(session);
  const drafts = draftsOf(session);
  const links = linksOf(read.thread);
  const openJob = (other: string) => go(jobPath(machine.origin, other));
  const onMenu = (entry: string) => {
    const job = { session_id: id };
    if (entry === "stop") void run(() => acts.stop(job));
    else if (entry === "pause") void run(() => acts.pause(job));
    else if (entry === "resume") void run(() => acts.resume(job));
    else if (entry === "end") void run(() => acts.close(job));
    else if (entry === "fork") void run(async () => openJob((await acts.fork(job)).id));
    else if (entry.startsWith("model:") || entry.startsWith("effort:")) {
      const model = entry.startsWith("model:") ? entry.slice("model:".length) : session.model;
      const effort = entry.startsWith("effort:") ? entry.slice("effort:".length) : session.effort;
      void run(() => acts.tune({ session_id: id, ...(model === undefined ? {} : { model }), ...(effort === undefined ? {} : { effort }), mode: session.mode ?? "auto" }));
    }
  };
  const planPending = approval !== undefined && !approval.approved && plan?.version !== undefined && approval.groups[0] !== undefined;
  const trailing = (
    <div className="pk-trailing">
      {asked === undefined && lapsed === undefined ? null : (
        <AskCards {...(asked === undefined ? {} : { asked, keep: keep(call ?? asked.command) })} onAnswer={onAnswer} {...(lapsed === undefined ? {} : { lapsed })} />
      )}
      {asksOf(session).map((item) => (
        <WaitingAsk key={item.id} item={item} keep={keep(`waiting:${item.id}`)} answer={(given) => run(() => acts.answerWaiting({ session_id: id, item_id: item.id, ...given }))} />
      ))}
      {runs.map(({ item, command }) => (
        <RunCard key={item.id} item={item} command={command} run={() => run(() => acts.run({ session_id: id, command }))} />
      ))}
      {drafts.map((draft) => (
        <DraftCard
          key={draft.id}
          draft={draft}
          keep={keep(`draft:${draft.id}:note`)}
          approve={() => run(() => acts.answerWaiting({ session_id: id, item_id: `draft:${draft.id}`, choice: 0 }))}
          sendBack={(note) => run(() => acts.answerWaiting({ session_id: id, item_id: `draft:${draft.id}`, text: note }))}
        />
      ))}
      {planPending ? (
        <PlanChanges
          keep={keep("plan")}
          send={(text) => run(() => acts.planChanges({ session_id: id, version: plan!.version!, marks: [{ kind: "note", id: `phone-${Date.now()}`, group: approval!.groups[0]!.id, text }] }))}
        />
      ) : null}
      <Trouble says={says} />
    </div>
  );
  const { state, said } = standing(
    asked !== undefined || lapsed !== undefined || planPending || asksOf(session).length > 0 || runs.length > 0 || drafts.length > 0,
    session.turn.state === "working",
    session.dead !== undefined,
  );
  const context = session.context;
  return (
    <section className="pk-screen" aria-label={session.title ?? "Job"}>
      <div className="deck-pane pk-pane" data-frame data-bare>
        <SessionFrame
          state={state}
          said={several ? `${said} · ${machine.name}` : said}
          id={id}
          {...(session.title === undefined ? {} : { title: session.title })}
          actions={<HeadFacts {...(links.from === undefined ? {} : { from: links.from })} started={links.started} {...(context === undefined ? {} : { context })} open={openJob} />}
          menu={{ entries: menuOf(session, models), onSelect: onMenu }}
          back={<Back label="Home" onClick={home} />}
        >
          <div className="armada-session-frame__centre">
            <SessionThread
              sessionId={`${machine.origin} ${id}`}
              rows={rowsOf(session)}
              {...(approval === undefined ? {} : { approval })}
              working={session.turn.state === "working"}
              onAnswer={onAnswer}
              onOpenSession={openJob}
              onOpenFile={(path) => go(filePath(machine.origin, id, path))}
              onOpenSketch={(sketch) => go(sketchPath(machine.origin, id, sketch))}
              trailing={trailing}
            />
            {session.dead !== undefined ? null : (
              <div className="pk-dock">
                <SessionComposer
                  bare
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
                  onSend={(sent) => run(() => acts.message({ session_id: id, text: sent.text.trim() }))}
                />
              </div>
            )}
          </div>
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
  }, [origin, id]);
  useEffect(() => {
    void start();
    setFound(origin);
  }, [origin]);
  const mac = found === undefined ? undefined : macOf(found);
  if (mac === undefined) {
    const says = lost ? "No paired Mac holds that Job." : fleet.loaded && found !== undefined ? "This phone is not paired with that Mac." : "";
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={() => go("/")} back="Home" />
        <div className="pk-body"><Trouble says={says} /></div>
      </section>
    );
  }
  return <Thread key={`${mac.machine.origin} ${id}`} machine={mac.machine} id={id} several={fleet.macs.length > 1} />;
}
