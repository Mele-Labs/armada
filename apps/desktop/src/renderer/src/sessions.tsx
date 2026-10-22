// Sessions, as the window draws them: one Session open in the Cockpit, and who owns
// a chip anywhere. Apart from `App.tsx`, which is at its length.
// What is drawn is `@armada/components`'; this reads the draft
// (`packages/screens/src/draft/sessions.ts`) into it.

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { ComponentProps, ReactNode } from "react";
import { createPortal } from "react-dom";
import { ExternalLink, GitFork, PanelRightClose, PanelRightOpen, PowerOff, Rewind } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import {
  Alert,
  BACKGROUND,
  Button,
  ChipOwnership,
  ContextFigure,
  ForkedFrom,
  AskCards,
  askLabel,
  askText,
  HandoffBand,
  NeedsYouPanel,
  type PlanApproval,
  SessionEndBand,
  TurnAccountBand,
  TurnAccountCard,
  TurnAccountPanel,
  type TurnAccount,
  type TurnLive,
  ConcernCard,
  DockedTerminal,
  ProseTerminal,
  Tooltip,
  Sheet,
  SessionComposer,
  quoteIntoDraft,
  SessionFrame,
  type DropdownMenuEntry,
  SessionLedger,
  SessionThread,
  RequestOpen,
  SavedOpen,
  type NoticeChoice,
  SketchPad,
  SketchPreview,
  SketchReading,
  StepsPanel,
  SubagentMarks,
  useCodeRun,
  useKept,
  useAccount,
  useTerminalDock,
  FILE_SHOWN,
} from "@armada/components";
import type { DebugPayload, RunsPlace } from "@armada/components";
import { PROTOCOL_ID, spoken } from "@armada/protocol";
import type {
  ProseTerminalValue,
  ChipOwnershipValue,
  FreshStart,
  LedgerEntry,
  OwnerChipRef,
  OwnerSummary,
  SketchBox,
  SketchLine,
  SketchPicture,
  SketchStroke,
  SessionThreadRow,
  RequestedThreadRow,
  SavedThreadRow,
  StepsPanelProps,
  RetroRowItem,
} from "@armada/components";
import { sessionOf } from "@armada/screens/src/lessons/retro";
import { askToOpenRetro } from "./open-retro";
import { useDispatchFailureHere } from "./failure-session";
import { handoffOf } from "./session-handoff";
import { chooseFolder, readRetro, searchFiles } from "./commands";
import { ArtifactBody, ARTIFACT_GLYPH, isAddress, PageFrame } from "./session-artifact";
import { accountPlace } from "./account-place";
import { isUnsentItem, linkEntry, unsentEntry, useStartedBy } from "./session-started";
import { readingOfSketch } from "./session-sketch";
import { forkAtPick, threadRowsOf, upTo } from "./session-thread-rows";
import { backgroundEntryOf } from "./session-background";
import { SubagentThread, useSubagentRead } from "./session-subagent";
import { useMinimized } from "./session-ledger-minimized";
import { useWriteClashes } from "./clashes";
import { showLayer } from "./session-view/layers";
import { OverTheApp } from "./session-view/OverTheApp";
import { baysOf } from "./session-view/slot-trouble";
import { ClashFileSheet } from "./session-view/ClashFileSheet";
import { helmOfferedOf, reachOfferedOf } from "@armada/screens/src/copy";
import { lapsedAnswerText } from "@armada/screens/src/sessions-wire";
import { attachmentsOf, ownerOf, owesOf, planAsked } from "@armada/screens/src/draft/sessions";
import type { ChipRef, DrawnSketch, Session, SessionAnswer, SessionAttachment, SessionMode, PlanMark, SessionsDraft, SessionTag } from "@armada/screens/src/draft/sessions";
import {
  isDrawn,
  nextPictureId,
  nextShapeId,
  withBody,
  withJoin,
  withPicture,
  withPlace,
  withShape,
  withStroke,
  withoutLastStroke,
  withoutShapes,
} from "@armada/screens/src/draft/sketch";
import type { Drawing } from "@armada/screens/src/draft/sketch";
import type { HeldWorktrees, Notice, RepositorySummary } from "@armada/protocol";
import { askToTell } from "./tell";
import { SessionRefs } from "./helm-refs";
import { openInPanel } from "./shared-panel/store";
import { useShownFiles } from "./session-shown-file";
import { startOf } from "./shared-panel/requests-model";
import { SURFACE, useAtFloor, useNarrow } from "@armada/shell";

import { useBackfilled } from "./backfilled";
import { ARTIFACT_SAID, cardOf, checksSaid, shownOnly, type Known } from "./session-cards";
import { useNumbered } from "./numbered";
import { RunsPlaceCard, RunsPlaceControl } from "./RunsPlaceControl";
import { composerFocusAsked } from "./session-focus";
import { NO_SYNCED, cardOn, profilesOf, useRunBook } from "./run-book";
import { boundProfile, rootOfSession, sentTo } from "./run-places";
import { SessionRepositoryOf } from "./session-move";
import { movedRowsOf } from "./session-moved";
import { useSessions, useSessionsDraft, useSessionsSaid } from "./sessions-draft";
import { restartOf } from "./build-change";
import { bangOf, hostedCommands, opensMcp, runnerOf } from "./session-runner";
import { todosOf } from "./session-activity";
import { dismissWaiting } from "./cockpit/dismissed";
import { holdsDelivery, stateOf } from "./session-state";
import type { PlanEditing } from "./session-view/PlanEdit";
import type { LoadoutAt } from "./session-loadout";
import { changeLines } from "./session-view/plan-marks";
import { planRevision } from "./session-view/plan-revision";
import { PlanReview } from "./session-view/PlanReview";
import { groupTitleOf, linkedPropsOf } from "./session-view/linked";
import { SessionWorkspace, type Hosts } from "./session-view/SessionWorkspace";
import { CallChip, callWord } from "./session-view/CallChip";
import { useLedgerRelease, type LedgerRelease } from "./session-ledger-release";

/** The surfaces the rail and the palette leave off: Sessions, which the Cockpit holds. ⌘3 and a link to a Session still arrive there. */
export function sessionsHidden(): readonly string[] {
  return [SURFACE.sessions];
}

type Checks = Extract<SessionAttachment, { kind: "pull_request" }>["checks"];

const redOf = (checks: Checks): { red?: "running" | "advisory" } =>
  checks.state === "failed" || checks.red === undefined ? {} : { red: checks.state === "pending" ? "running" : "advisory" };

const pullRequestsOf = (session: Session) =>
  attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks), ...redOf(one.checks) }));

/** A live view's frame has nothing to close. */
const NO_CLOSE = (): void => undefined;

function summaryOf(session: Session): OwnerSummary {
  return {
    id: session.id,
    ...(session.address === undefined ? {} : { address: session.address }),
    ...(session.title === undefined ? {} : { title: session.title }),
    ...stateOf(session),
    slots: attachmentsOf(session, "slot").map((one) => one.slot),
    pullRequests: pullRequestsOf(session),
    ...(session.lastTurnAt === undefined ? {} : { lastTurnAt: session.lastTurnAt }),
  };
}

/** A Session just started, opened once its id is known. A start Fleet refused says why in `said` and opens nothing. */
function opening(started: string | Promise<string | undefined>, onOpen: (id: string) => void): void {
  if (typeof started === "string") onOpen(started);
  else void started.then((id) => id !== undefined && onOpen(id));
}

/** A Session this large, or this old, is offered a fresh start. */
const FRESH_TOKENS = 300_000;
const FRESH_HOURS = 24;

/** A new Session under the same title, whose first message is the handoff `text`. */
async function startFreshFrom(draft: SessionsDraft, session: Session, text: string): Promise<string | undefined> {
  const fresh = await draft.start();
  if (fresh === undefined) return undefined;
  if (session.title !== undefined) draft.rename?.(fresh, session.title);
  void draft.send(fresh, { text, files: [], sketches: [], tags: [] });
  return fresh;
}

/** Who owns each chip, for every chip in the window. */
export function SessionsOwnership({ onOpen, children }: { onOpen: (sessionId: string) => void; children: ReactNode }) {
  const sessions = useSessions();
  const value = useMemo<ChipOwnershipValue>(
    () => ({
      ownerOf: (chip: OwnerChipRef) => {
        const owner = ownerOf(sessions, chip as ChipRef);
        return owner === undefined ? undefined : summaryOf(owner);
      },
      open: onOpen,
    }),
    [sessions, onOpen],
  );
  return <ChipOwnership.Provider value={value}>{children}</ChipOwnership.Provider>;
}

/** What Fleet refused the last act on a Session, in the words it gave. */
function Refused() {
  const said = useSessionsSaid();
  return said === undefined || said === "" ? null : <Alert tone="escalated">{said}</Alert>;
}

/** The Sessions a Retro press is out for, shared by every place that draws the press so a second view is held too. */
const pressing = new Set<string>();
let pressingNow: ReadonlySet<string> = new Set();
const pressingHeard = new Set<() => void>();

function pressed(id: string, on: boolean): void {
  if (on) pressing.add(id);
  else pressing.delete(id);
  pressingNow = new Set(pressing);
  pressingHeard.forEach((one) => one());
}

const hearPressing = (one: () => void) => {
  pressingHeard.add(one);
  return () => void pressingHeard.delete(one);
};

/** Whether the retro in this Session's thread is still being written. A Session that ended or went quiet is not writing one. */
const writingIn = (session: Session): boolean =>
  session.dead === undefined && session.rows.some((row) => row.kind === "retro" && row.stage !== "written" && row.stage !== "failed");

/**
 * The Retro press: which Sessions have a retro being written, and the press. **It does not wait on
 * the answer and it moves nobody.** The retro is a card in the Session's thread, written there as
 * Fleet goes (`SessionRetroRow`), and its `Open retro` link is the one way it opens on the Retros
 * page, so a person watching the thread is not taken from it. A terminal Session's thread is a
 * transcript with no such row, so its press keeps the mark and opens the retro when Fleet answers.
 * The mark holds while the press is out
 * and while the thread's own row says Fleet is still at it; Fleet's refusal, a second press while one
 * is being written, is worded by the Sessions page and the row carries on.
 */
function useRetroPress(): { writing: ReadonlySet<string>; press?: (session: { id: string; title?: string; terminal?: true }, watching?: boolean) => void } {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const out = useSyncExternalStore(hearPressing, () => pressingNow);
  const writing = useMemo(() => new Set([...out, ...sessions.filter(writingIn).map((one) => one.id)]), [out, sessions]);
  // What the thread holds when the answer comes, not when the press was made.
  const latest = useRef(sessions);
  latest.current = sessions;
  const retro = draft?.retro;
  if (retro === undefined) return { writing };
  const retroIds = (id: string) => new Set(latest.current.find((one) => one.id === id)?.rows.flatMap((row) => (row.kind === "retro" ? [row.id] : [])));
  return {
    writing,
    press: ({ id, title, terminal }, watching = false) => {
      const before = retroIds(id);
      pressed(id, true);
      void retro(id)
        .then((written) => {
          // **Nothing else says it is written** where there is no card to watch: a terminal Session's thread is a
          // transcript, and a press answered with the last retro (nothing new since) adds no row. Then the answer
          // opens the retro, as the press always did. A thread this window is not watching has no card to look at either way.
          const card = [...retroIds(id)].some((one) => !before.has(one));
          if (written && (terminal === true || (watching && !card))) askToOpenRetro({ id, label: sessionOf({ id, ...(title === undefined ? {} : { title }) }) });
        })
        .finally(() => pressed(id, false));
    },
  };
}

/**
 * A thread's rows, with each written retro's items filled in. **The row Fleet keeps says only which
 * retro was written**, so the count and the titles under `Retro 3 written` are read from the retro
 * itself, once, the first time this thread draws it.
 */
/** What a Disable choice covers: this Job, its repository by the Manifest the Job holds, or this Job for an hour. */
function muteNoticeFor(mute: NonNullable<SessionsDraft["muteNotice"]>, session: Session, notice: Notice, choice: NoticeChoice): void {
  if (choice !== "repo") return void mute(notice, { kind: choice, session_id: session.id });
  if (session.manifestId === undefined) return askToTell("Fleet serves no Manifest for that repository.");
  void mute(notice, { kind: "repo", manifest_id: session.manifestId });
}

function useThreadRows(session: Session): SessionThreadRow[] {
  const [kept, setKept] = useState<ReadonlyMap<string, readonly RetroRowItem[]>>(new Map());
  const asked = useRef(new Set<string>());
  const { id } = session;
  const rows = threadRowsOf(session);
  const wanted = rows.flatMap((row) => (row.kind === "retro" && row.stage === "written" && row.retro !== undefined && row.items.length === 0 ? [row.retro] : []));
  const key = wanted.join(",");
  useEffect(() => {
    for (const n of key === "" ? [] : key.split(",").map(Number)) {
      const at = `${id}:${n}`;
      if (asked.current.has(at)) continue;
      asked.current.add(at);
      void readRetro({ kind: "session", id, n }).then((read) => {
        if (!read.ok) return void asked.current.delete(at);
        const items = (read.retro.items ?? []).map((one): RetroRowItem => ({ ...(one.title === undefined ? {} : { title: one.title }), ...(one.what === undefined ? {} : { what: one.what }) }));
        setKept((was) => new Map(was).set(at, items));
      });
    }
  }, [id, key]);
  if (kept.size === 0) return rows;
  return rows.map((row) => {
    const items = row.kind === "retro" && row.retro !== undefined && row.items.length === 0 ? kept.get(`${id}:${row.retro}`) : undefined;
    return items === undefined ? row : { ...row, items };
  });
}

/** Opens a retro this Session wrote on the Retros page: its number, named by the Session's title and address. Stable while the Session's name is. */
function useOpenRetro(session: Session): (retro: number) => void {
  const { id, title } = session;
  return useCallback((n) => askToOpenRetro({ id, n, label: sessionOf({ id, ...(title === undefined ? {} : { title }) }) }), [id, title]);
}

/** Where a ledger row goes: the surface that already shows the thing. */
export type LedgerGoes = {
  onGoTo: (surfaceId: string) => void;
  onOpenLink: (address: string) => void;
  onOpenCheck: (sessionId: string, run: number) => void;
  /** Opens the Session's Loadout in the panel over the work area, where a Check's opens; `at` is the group it opens on. */
  onOpenLoadout: (sessionId: string, at?: LoadoutAt) => void;
};

/** What the ledger opens beside it, where no surface of Bridge's draws the thing: a sketch, a subagent, an artifact. */
export type Reading = { kind: "sketch"; id: string } | { kind: "subagent"; id: string } | { kind: "background"; id: string } | { kind: "artifact"; id: string } | { kind: "steps"; id: string };

/** What a press on a ledger row asks to read: a pull request opens in the side panel, the rest beside the thread. */
export type ReadAsked = Reading | { kind: "pull_request"; number: number };

export const readingOf = (session: Session, open: Reading | undefined): SessionAttachment | undefined =>
  open === undefined || open.kind === "steps" ? undefined : session.attachments.find((one) => one.kind === open.kind && "id" in one && one.id === open.id);

/** A waiting item's steps as the panel beside the thread draws them: the item is not an attachment, so it has its own reading. */
export type StepsReading = Omit<StepsPanelProps, "onOpenInBrowser">;

export function stepsReadingOf(session: Session, open: Reading | undefined, draft: SessionsDraft | undefined): StepsReading | undefined {
  if (open?.kind !== "steps") return undefined;
  const item = owesOf(session).find((one) => one.id === open.id);
  if (item?.steps === undefined) return undefined;
  const address = item.act?.kind === "walk" ? item.act.target : undefined;
  const command = item.act?.kind === "run" ? item.act.target : undefined;
  return {
    name: item.text,
    ...(address === undefined ? {} : { address }),
    ...(command === undefined ? {} : { command }),
    steps: item.steps,
    keep: `steps:${session.id}:${item.id}`,
    onOpenAddress: (url) => draft?.openWindow?.(session.id, url),
    onSend: (text) => draft?.answerWaiting?.(session.id, item.id, { text }),
  };
}

export function ReadingSheet({
  sessionId,
  readSubagent,
  one,
  steps,
  draft,
  onClose,
  onOpenLink,
}: {
  sessionId: string;
  readSubagent: SessionsDraft["subagent"];
  one: SessionAttachment | undefined;
  /** An item's steps, read in place of an attachment. */
  steps?: StepsReading;
  draft: SessionsDraft | undefined;
  onClose: () => void;
  onOpenLink: (address: string) => void;
}) {
  const floor = useAtFloor();
  const subagent = one?.kind === "subagent" ? one : undefined;
  const subagentThread = useSubagentRead(sessionId, subagent, readSubagent);
  const title =
    steps !== undefined
      ? steps.name
      : one === undefined
      ? ""
      : one.kind === "sketch"
        ? `Sketch ${one.title}`
        : one.kind === "subagent"
          ? `Subagent ${one.task}`
          : one.kind === "background"
            ? one.task
            : one.kind === "artifact"
              ? one.title
              : "";
  const artifact = one?.kind === "artifact" ? one : undefined;
  // The agent's sketch is a scene, which wants the width.
  const drawnByAgent = one?.kind === "sketch" && (one.by === "agent" || "nodes" in one.drawing);
  const background = one?.kind === "background" ? one : undefined;
  const Glyph = artifact !== undefined ? ARTIFACT_GLYPH[artifact.form] : background !== undefined ? BACKGROUND[background.form].Glyph : undefined;
  return (
    <OverTheApp open={(one !== undefined || steps !== undefined) && title !== ""}>
    <Sheet
      kind="session-reading"
      open={(one !== undefined || steps !== undefined) && title !== ""}
      floating
      floor={floor}
      title={title}
      {...(drawnByAgent || subagent !== undefined ? { size: "wide" as const } : {})}
      {...(subagent?.agentType === undefined && subagentThread?.model === undefined ? {} : { subtitle: <SubagentMarks type={subagent?.agentType} model={subagentThread?.model} /> })}
      bleed={artifact !== undefined || background !== undefined || one?.kind === "subagent" || drawnByAgent}
      {...(Glyph === undefined ? {} : { leading: <Glyph size={16} strokeWidth={2} aria-hidden /> })}
      {...(subagentThread?.context === undefined ? {} : { controls: <ContextFigure fill={subagentThread.context} /> })}
      {...(artifact !== undefined && isAddress(artifact.form)
        ? {
            controls: (
              <Tooltip label="Open in browser">
                <Button variant="ghost" size="sm" aria-label="Open in browser" onClick={() => onOpenLink(artifact.id)}>
                  <ExternalLink size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            ),
          }
        : {})}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      {steps !== undefined ? (
        // The sheet's body pads it already: a second pad doubled the margin.
        <StepsPanel {...steps} onOpenInBrowser={onOpenLink} />
      ) : one?.kind === "sketch" ? (
        one.by === "agent" || "nodes" in one.drawing ? (
          <SketchReading {...readingOfSketch(one)} />
        ) : (
          <SketchPreview label={one.title} boxes={one.drawing.boxes} lines={one.drawing.lines} strokes={one.drawing.strokes ?? []} pictures={[]} />
        )
      ) : one?.kind === "subagent" ? (
        <SubagentThread sessionId={sessionId} one={one} thread={subagentThread} />
      ) : one?.kind === "background" ? (
        // The output is a file the agent's shell wrote, read as an artifact file is.
        one.output === undefined ? null : <ArtifactBody draft={draft} sessionId={sessionId} form="file" id={one.output} title={one.task} onClose={onClose} />
      ) : one?.kind === "artifact" ? (
        <ArtifactBody draft={draft} sessionId={sessionId} form={one.form} id={one.id} title={one.title} onClose={onClose} />
      ) : null}
    </Sheet>
    </OverTheApp>
  );
}

const BLANK: Drawing = { shapes: [], joins: [], strokes: [], pictures: [] };

/** The pad Dispatch draws on, in a sheet over the Session: what is drawn goes with the next message. */
export function SketchSheet({ open, onClose, onAttach }: { open: boolean; onClose: () => void; onAttach: (sketch: DrawnSketch) => void }) {
  const floor = useAtFloor();
  const [drawing, setDrawing] = useState<Drawing>(BLANK);
  const attach = () => {
    const words = drawing.shapes.map((one) => one.body.trim()).find((one) => one !== "");
    onAttach({
      id: `d${Date.now()}`,
      title: words === undefined ? "Sketch" : words.slice(0, 32),
      drawing: {
        boxes: drawing.shapes.map(({ id, x, y, body }) => ({ id, x, y, body })),
        lines: drawing.joins.map(({ id, from, to }) => ({ id, from, to })),
        strokes: drawing.strokes.map(({ id, points }) => ({ id, points })),
      },
    });
    setDrawing(BLANK);
  };
  return (
    <OverTheApp open={open}>
    <Sheet kind="session-sketch" open={open} floating floor={floor} size="wide" title="Draw a sketch" closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="armada-session-reading">
        <SketchPad
          label="Sketch for the message"
          boxes={drawing.shapes.map(({ id, x, y, body }): SketchBox => ({ id, x, y, body }))}
          lines={drawing.joins.map(({ id, from, to }): SketchLine => ({ id, from, to }))}
          strokes={drawing.strokes.map(({ id, points }): SketchStroke => ({ id, points }))}
          pictures={drawing.pictures.map(({ id, x, y, width, height, src }): SketchPicture => ({ id, x, y, width, height, src }))}
          onAdd={(at, body) => setDrawing((one) => withShape(one, { id: nextShapeId(one), x: at.x, y: at.y, body }))}
          onPicture={(picture, at) => setDrawing((one) => withPicture(one, { id: nextPictureId(one), x: at.x, y: at.y, ...picture }))}
          onBody={(id, body) => setDrawing((one) => withBody(one, id, body))}
          onMove={(id, at) => setDrawing((one) => withPlace(one, id, at))}
          onRemove={(ids) => setDrawing((one) => withoutShapes(one, ids))}
          onJoin={(from, to) => setDrawing((one) => withJoin(one, from, to))}
          onDraw={(points) => setDrawing((one) => withStroke(one, points))}
          onUndo={() => setDrawing(withoutLastStroke)}
        />
        <Button variant="primary" size="sm" disabled={!isDrawn(drawing)} onClick={attach}>
          Attach sketch
        </Button>
      </div>
    </Sheet>
    </OverTheApp>
  );
}

type Runner = NonNullable<LedgerEntry["run"]>["runner"];

export function entriesOf(
  session: Session,
  goes: LedgerGoes,
  read: (one: ReadAsked) => void,
  slot: (n: number) => void,
  sessions: readonly Session[],
  open: (id: string) => void,
  openWindow: (url: string) => void,
  answerWaiting: ((itemId: string, answer: { choice: number } | { text: string }) => void | Promise<unknown>) | undefined,
  scrollToAsk: () => void,
  known: Known,
  runner: Runner | undefined,
  removeWaiting?: (itemId: string, reason: string) => void,
  release?: LedgerRelease["rowOf"],
): LedgerEntry[] {
  // An ended Session waits on nobody: its ledger is what it made, to be read.
  // A recorded plan and a judge's concern are cards in the thread, so the ledger does not list them again.
  const waiting = (session.dead === "ended" ? [] : owesOf(session).filter((one) => one.source !== "plan" && one.source !== "judge")).map((one): LedgerEntry => {
    if (isUnsentItem(one)) return unsentEntry(one, scrollToAsk);
    // An `answer` with no choices and no question in the thread has nowhere to go, so it takes words as no act does.
    const act = one.act?.kind === "answer" && one.options === undefined && session.asked === undefined ? undefined : one.act;
    // A restart is the Fleet panel's build switch: it reads as its label and is one key, as on its call card.
    const restart = restartOf(act);
    return {
      key: `waiting${one.id}`,
      kind: "waiting",
      name: restart?.label ?? one.text,
      // A procedure reads as its words: its command is in the steps it opens.
      text: restart !== undefined ? restart.label : act?.kind === "run" && one.steps === undefined ? <code>{act.target}</code> : one.text,
      ...(restart !== undefined
        ? {}
        : {
            card: {
              title: one.text,
              facts: act?.kind === "run" ? [{ label: "Command", value: act.target, mono: true }] : act?.kind === "walk" ? [{ label: "Page", value: act.target, mono: true }] : [],
            },
          }),
      ...(one.steps !== undefined ? { act: "steps" as const } : act === undefined ? {} : { act: act.kind }),
      ...(restart !== undefined && act !== undefined ? { restart: { command: act.target, ...(runner === undefined ? {} : { runner }) } } : {}),
      ...(act?.kind === "run" && restart === undefined && runner !== undefined ? { run: { command: act.target, runner } } : {}),
      ...(one.options === undefined || answerWaiting === undefined
        ? {}
        : {
            options: one.options.map((option, at) => ({
              label: option.label,
              ...(option.description === undefined ? {} : { description: option.description }),
              onPick: () => answerWaiting(one.id, { choice: at }),
            })),
            onOther: (text: string) => answerWaiting(one.id, { text }),
          }),
      ...(act !== undefined || one.steps !== undefined || (one.source !== "agent" && one.source !== "replied") || answerWaiting === undefined
        ? {}
        : { onReply: (text: string) => answerWaiting(one.id, { text }), replyKeep: `waiting:${session.id}:${one.id}` }),
      // An item the agent stated is the person's to take off; the agent is told, with his reason where he gave one.
      ...(one.source !== "agent" || removeWaiting === undefined ? {} : { onRemove: (reason: string) => removeWaiting(one.id, reason), removeKeep: `remove:${session.id}:${one.id}` }),
      onOpen: () => {
        if (one.steps !== undefined) return read({ kind: "steps", id: one.id });
        if (act === undefined || restart !== undefined) return;
        if (act.kind === "walk") openWindow(act.target);
        else if (act.kind === "approve_pr") read({ kind: "pull_request", number: Number(act.target.replace(/^#/, "")) });
        else if (act.kind === "run") {
          try {
            void navigator.clipboard.writeText(act.target);
          } catch {
            // Not copied: the command stays on the row to be read.
          }
        } else scrollToAsk();
      },
    };
  });
  // A slot a branch row already carries as its chip has no row of its own.
  const carried = new Set(attachmentsOf(session, "branch").map((one) => one.slot));
  // An issue is drawn in Progress, with its pull requests under it, and has no row here.
  const drawn = session.attachments.filter((one): one is Exclude<SessionAttachment, { kind: "issue" }> => one.kind !== "issue" && (one.kind !== "slot" || !carried.has(one.slot)));
  return [
    ...waiting,
    ...drawn.map((one): LedgerEntry => {
      const entry = entryOf(one);
      return { ...entry, ...release?.(one, entry), card: cardOf(one, session, sessions, known) };
    }),
  ];

  function entryOf(one: (typeof drawn)[number]): Omit<LedgerEntry, "card"> {
    switch (one.kind) {
      case "started_to": {
        const child = one.group === undefined ? undefined : sessions.find((other) => other.id === one.id);
        if (child !== undefined) {
          const props = linkedPropsOf(child, open, groupTitleOf(session, one.group!));
          return { key: `linked${child.id}`, kind: "linked", name: `${props.title}, ${props.said}`, text: props.title, linked: props, onOpen: props.onOpen };
        }
        return linkEntry(one, sessions, open);
      }
      case "forked_to":
      case "forked_from":
      case "started_by":
        return linkEntry(one, sessions, open);
      case "slot":
        return {
          key: `slot${one.slot}`,
          kind: "slot",
          name: `Worktree slot ${one.slot}`,
          text: <code>{one.slot}</code>,
          onOpen: () => slot(one.slot),
        };
      case "branch":
        return {
          key: one.name,
          kind: "branch",
          name: `Branch ${one.name}`,
          text: <code>{one.name}</code>,
          ...(one.slot === 0 ? {} : { slot: one.slot }),
          onOpen: () => slot(one.slot),
        };
      case "pull_request":
        return {
          key: String(one.number),
          kind: "pull_request",
          name: `Pull request #${one.number}, checks ${one.checks.state}`,
          text: (
            <>
              <code>#{one.number}</code> {one.title}
            </>
          ),
          mark: { glyph: one.state === "merged" ? "passed" : one.checks.state, said: one.state === "merged" ? "Merged" : checksSaid(one.checks) },
          pr: { merged: one.state === "merged", checks: one.checks.state === "failed" ? "failed" : one.checks.state === "pending" ? "running" : "passed", checksSaid: checksSaid(one.checks) },
          ...(one.state === "merged" ? { finished: true } : {}),
          onOpen: () => read({ kind: "pull_request", number: one.number }),
        };
      case "studio":
        return { key: one.id, kind: "studio", name: `Studio ${one.title}`, text: one.title, onOpen: () => goes.onGoTo(SURFACE.studios) };
      case "sketch":
        return { key: one.id, kind: "sketch", name: `Sketch ${one.title}`, text: one.title, onOpen: () => read({ kind: "sketch", id: one.id }) };
      case "background":
        return backgroundEntryOf(one, () => read({ kind: "background", id: one.id }));
      case "artifact":
        return {
          key: one.id,
          kind: "artifact",
          artifact: one.form,
          name: `${shownOnly(one) ? FILE_SHOWN : ARTIFACT_SAID[one.form]} ${one.title}`,
          ...(shownOnly(one) ? { shown: true as const } : {}),
          text: one.title,
          // A file the Session wrote opens in the side panel's editor; a page or a doc stays in the reading sheet.
          onOpen: () => (one.form === "window" ? openWindow(one.id) : one.form === "file" && openInPanel({ kind: "file", path: one.id, from: "session", sessionId: session.id }) ? undefined : read({ kind: "artifact", id: one.id })),
        };
      case "subagent":
        return {
          key: one.id,
          kind: "subagent",
          name: `Subagent ${one.task}, ${one.state}`,
          text: one.task,
          mark: { glyph: one.state, said: one.state === "running" ? "Running" : "Done" },
          ...(one.state === "running" ? {} : { finished: true }),
          onOpen: () => read({ kind: "subagent", id: one.id }),
        };
    }
  }
}

/** What Cleanup holds, read here for a slot's panel. */
export type HeldReads = { held: HeldWorktrees; onWant: (want: boolean) => void };

/** An act in the header's menu: its glyph, its label and what it does. */
export type DeckAct = { id: string; label: string; icon: LucideIcon; run: () => void };

/**
 * What the flight deck changes in a Session it draws on its stage: its own acts beside the Session's, and the ledger drawn where
 * the deck keeps it, in the right column, and not beside the thread. There the ledger is always up, so the fold below the
 * breakpoint and the button that hides it are not drawn.
 */
export type DeckSession = {
  /** The deck's acts, first in the header's menu: Open the Session page and Stop. */
  acts: readonly DeckAct[];
  /** The way back, the header's far right item. */
  back: ReactNode;
  /** The element the ledger is drawn into. Absent until the deck has drawn it, and the ledger waits for it. */
  ledger: HTMLElement | null;
  /** The deck's places a Session with `work` draws its own panels into, in place of Activity, Bays and Autopilot. */
  hosts?: Hosts;
};

/** A hand off that did not start as the failure toast carries a failure to a new Job: the code is Fleet's attachment kind, the reason is `detail.reason`. */
function handoffFailure(session: Session): DebugPayload {
  return {
    code: "handoff_failed",
    message: "Failed to hand off to new job",
    fields: [
      { key: "session_id", value: session.id },
      ...(session.handoffReason === undefined ? [] : [{ key: "reason", value: session.handoffReason }]),
    ],
    bridgeProtocol: spoken(PROTOCOL_ID),
    at: new Date().toISOString(),
  };
}

/** One Session open: its conversation in the middle and what it holds at the side, one panel. */
const NO_SKETCHES: DrawnSketch[] = [];
const NO_ROWS: SessionThreadRow[] = [];

const NO_REPOSITORIES: readonly RepositorySummary[] = [];

export function SessionView({ session, goes, onOpen, held, deck, repositories = NO_REPOSITORIES, trailing }: { session: Session; goes: LedgerGoes; onOpen: (id: string) => void; held: HeldReads; deck?: DeckSession; repositories?: readonly RepositorySummary[]; trailing?: ReactNode }) {
  const draft = useSessionsDraft();
  const runBook = useRunBook();
  const account = useAccount();
  const sessions = useSessions();
  const clashes = useWriteClashes();
  const numbered = useNumbered(session.manifestId);
  const floor = useAtFloor();
  const [reading, setReading] = useState<Reading | undefined>();
  /** The plan opened in full to review. */
  const [reviewing, setReviewing] = useState(false);
  // What he marked on the plan, held until he sends it: the review overlay's and the Plan panel's alike.
  const [marks, setMarks] = useState<PlanMark[]>([]);
  useEffect(() => setMarks([]), [session.id]);
  // A file the agent shows opens in the panel while this Session is the one on screen.
  useShownFiles(session);
  const [padOpen, setPadOpen] = useState(false);
  const [drawn, setDrawn] = useKept<DrawnSketch[]>(`session:${session.id}:drawn`, NO_SKETCHES);
  const narrow = useNarrow() && deck === undefined;
  const [ledgerOpen, setLedgerOpen] = useState(false);
  // The Session a fork from a selection opened, whose message box takes focus when it shows.
  // A Reply asks again with a new `at`, for a box that is already in view.
  const [focusFor, setFocusFor] = useState<string | { key: string; at: number } | undefined>();
  // A Job opened with nothing in it takes the caret as its page shows.
  useEffect(() => {
    if (composerFocusAsked(session.id)) setFocusFor({ key: session.id, at: Date.now() });
  }, [session.id]);
  // The old Session is writing the handoff a fresh one opens on.
  const [handing, setHanding] = useState(false);
  // Whether the Needs you panel on the composer is up, and for which Session.
  const [panel, setPanel] = useState<{ id: string; open: boolean } | undefined>();
  const [endPanel, setEndPanel] = useState<{ id: string; open: boolean } | undefined>();
  useEffect(() => setEndPanel(undefined), [session.id]);
  const [nowPanel, setNowPanel] = useState<{ id: string; open: boolean } | undefined>();
  const [failedAway, setFailedAway] = useState<string | undefined>();
  const dispatchFailure = useDispatchFailureHere();
  const centre = useRef<HTMLDivElement>(null);
  // The thread's height, so a panel over it can rise to nearly all of it and no further.
  useEffect(() => {
    const stage = centre.current?.querySelector<HTMLElement>(".armada-session-thread__stage");
    if (stage === undefined || stage === null) return;
    const watch = new ResizeObserver(() => centre.current?.style.setProperty("--thread-h", `${stage.clientHeight}px`));
    watch.observe(stage);
    return () => watch.disconnect();
  }, [session.id]);
  // Keep going hides the callout for the `done` it answered; a later `done` is a new one.
  const [kept, setKept] = useState<string | undefined>();
  // The lapsed question answered from here, hidden until Fleet's thread shows the message that settles it.
  const [answeredLapse, setAnsweredLapse] = useState<string | undefined>();
  const [minimized, minimize] = useMinimized();
  const retro = useRetroPress();
  const threadRows = useThreadRows(session);
  const openRetro = useOpenRetro(session);
  const { onWant } = held;
  const { id } = session;
  const thread = useBackfilled(threadRows, id);
  const released = useLedgerRelease(session, draft);
  const watch = draft?.watch;
  const refresh = draft?.refresh;
  // A real Fleet reads a thread when it is opened and follows it from there.
  useEffect(() => watch?.(id), [watch, id]);
  // The Checks on a pull request are the forge's now, and the ledger holds what was last read.
  const prs = attachmentsOf(session, "pull_request").map((one) => one.number).join(",");
  useEffect(() => {
    for (const number of prs === "" ? [] : prs.split(",")) refresh?.(id, Number(number));
  }, [refresh, id, prs]);
  // The panel a slot's tile opens on Cleanup reads what Fleet holds, so it is wanted while this is open.
  useEffect(() => {
    onWant(true);
    return () => onWant(false);
  }, [onWant]);
  const { state, said } = stateOf(session);
  // The conversation a fork began as a copy of, which is the other Session's own rows.
  const forkedFrom = session.attachments.find((one): one is Extract<SessionAttachment, { kind: "forked_from" }> => one.kind === "forked_from");
  const origin = forkedFrom === undefined ? undefined : sessions.find((one) => one.id === forkedFrom.id);
  // **Up to and including the message it was forked at**, and not what was said after it.
  const startedBy = useStartedBy(session, sessions, onOpen);
  const originRows = origin === undefined ? NO_ROWS : upTo(threadRowsOf(origin), forkedFrom?.at);
  // **A move carries its whole conversation into the thread**, under a row naming where it came from, and ends the old one's with where it went.
  const carried = forkedFrom?.moved !== undefined;
  const carriedFrom = carried ? forkedFrom?.id : undefined;
  useEffect(() => {
    if (carriedFrom !== undefined) watch?.(carriedFrom);
  }, [watch, carriedFrom]);
  // **A Job that took another's place carries that Job's turns above its own**, over a plain rule: one scroll.
  const earlier = session.replaces === undefined ? undefined : sessions.find((one) => one.id === session.replaces);
  const shown = useMemo(() => {
    if (earlier !== undefined) {
      const before = threadRowsOf(earlier).map((row): SessionThreadRow => ({ ...row, id: `${earlier.id}:${row.id}` }));
      return [...before, { id: `divider-${earlier.id}`, at: "", kind: "divider" } as SessionThreadRow, ...thread];
    }
    const { head, tail } = movedRowsOf(session, repositories);
    const before = !carried || origin === undefined ? NO_ROWS : originRows.map((row): SessionThreadRow => ({ ...row, id: `${origin.id}:${row.id}` }));
    return head.length + before.length + tail.length === 0 ? thread : [...head, ...before, ...thread, ...tail];
    // eslint-disable-next-line react-hooks/exhaustive-deps -- originRows is derived from origin and the fork's `at`.
  }, [session.attachments, repositories, thread, carried, origin, forkedFrom?.at, earlier]);
  // A request in the thread opens in Requests for this Job, to edit and send again.
  const openRequest = useCallback((row: RequestedThreadRow) => void openInPanel({ kind: "requests", sessionId: session.id, start: startOf(row), from: row.id }), [session.id]);
  const openSaved = useCallback((row: SavedThreadRow) => void openInPanel({ kind: "requests", sessionId: session.id, saved: { collection: row.collectionId, request: row.request }, from: row.id }), [session.id]);
  // A run drawn as a terminal: the pane is the view's own, docked above its box.
  const runner = draft === undefined ? undefined : runnerOf(draft, session);
  const dock = useTerminalDock(useCodeRun(runner), session.id);
  if (draft === undefined) return null;
  const mode: SessionMode = session.mode ?? (session.terminal === true ? "ask" : "auto");
  // Folded below the breakpoint, a press on a row closes the ledger's sheet first, so what it opens is not drawn over it.
  const fold = <T extends unknown[]>(open: (...args: T) => void) => (...args: T) => {
    setLedgerOpen(false);
    open(...args);
  };
  const terminal: ProseTerminalValue | null = draft.terminalRuns === undefined ? null : { dock };
  const hours = session.trail === undefined ? 0 : Math.floor((Date.now() - Date.parse(session.trail.started)) / 3_600_000);
  const tokens = session.context?.tokens ?? 0;
  const lastRow = thread[thread.length - 1];
  const fresh: FreshStart | undefined =
    session.dead === undefined && (tokens > FRESH_TOKENS || hours >= FRESH_HOURS)
      ? {
          tokens,
          hours,
          busy: handing,
          onFresh: () => {
            setHanding(true);
            void handoffOf(draft, session)
              .then((text) => startFreshFrom(draft, session, text))
              .then((id) => id !== undefined && onOpen(id))
              .finally(() => setHanding(false));
          },
          ...(draft.forkAt === undefined || lastRow === undefined ? {} : { onFork: () => opening(draft.forkAt!(session.id, lastRow.id), onOpen) }),
        }
      : undefined;
  /** The Cockpit draws this Session's panels itself, the Ledger among them, so no second one is drawn beside the thread or in the deck's right column. */
  const workspace = deck?.hosts !== undefined && session.work !== undefined;
  const planNow = session.work?.plan;
  const planEditing: PlanEditing | undefined =
    planNow === undefined
      ? undefined
      : {
          id: session.id,
          plan: planNow,
          marks,
          set: setMarks,
          send: () => {
            draft.work?.suggest(session.id, marks);
            setMarks([]);
            setReviewing(false);
          },
        };
  /** Approve, with what he marked on the plan going to the agent in the same press. */
  const approvePlan = () => {
    setReviewing(false);
    if (marks.length === 0) return draft.work?.approve(session.id);
    draft.work?.suggest(session.id, marks, true);
    setMarks([]);
  };
  // A pull request of the Session opens in the side panel; the other readings open beside the thread.
  const read = (one: ReadAsked) => {
    if (one.kind !== "pull_request") return setReading(one);
    const address = attachmentsOf(session, "pull_request").find((pr) => pr.number === one.number)?.address;
    openInPanel({ kind: "pr", number: one.number, address: address ?? "" });
  };
  // A Session with no repository of its own opens the slot the pool names that number, as the sheet it replaced found it.
  const openSlot = (n: number) => {
    const pool = held.held.state === "read" ? (held.held.held.slots ?? []) : [];
    const manifestId = session.manifestId ?? pool.find((one) => one.slot === n)?.manifest_id;
    if (manifestId !== undefined) openInPanel({ kind: "bay", manifestId, slot: n });
  };
  const bays = baysOf(session, held.held.state === "read" ? (held.held.held.slots ?? []) : [], clashes);
  // A stranded bay is a call in the thread. A bay well behind its base and a write clash light the Bays key and raise no call: Fleet
  // tells the agent, so there is nothing in either for him to do.
  const entries: LedgerEntry[] = [
    ...(session.dead === "ended"
      ? []
      : bays.bays.flatMap((bay) =>
          bay.wrong !== "stranded" || bay.said === undefined ? [] : [{ key: `waiting-slot-${bay.slot.slot}`, kind: "waiting" as const, name: bay.said, text: bay.said, onOpen: () => showLayer(session.id, "slot") }],
        )),
    ...entriesOf(
    released.shown,
    narrow ? { ...goes, onGoTo: fold(goes.onGoTo) } : goes,
    narrow ? fold(read) : read,
    narrow ? fold(openSlot) : openSlot,
    sessions,
    narrow ? fold(onOpen) : onOpen,
    (url) => draft.openWindow?.(id, url),
    draft.answerWaiting === undefined ? undefined : (itemId, answer) => draft.answerWaiting?.(id, itemId, answer),
    narrow
      ? () => {
          setLedgerOpen(false);
          setPanel({ id: session.id, open: true });
        }
      : () => setPanel({ id: session.id, open: true }),
    { slots: held.held.state === "read" ? (held.held.held.slots ?? []) : [] },
    runner,
    (itemId, reason) => void dismissWaiting(id, itemId, reason),
    released.rowOf,
    ),
  ];
  // What the done callout lists: every pull request, merged first then open, else the pages, files and Docs made. Pictures looked at and windows shown were not made.
  const prRows = entries.filter((one) => one.kind === "pull_request").sort((a, b) => Number(b.pr?.merged === true) - Number(a.pr?.merged === true));
  const made = prRows.length > 0 ? prRows : entries.filter((one) => one.kind === "artifact" && (one.artifact === "page" || one.artifact === "file" || one.artifact === "doc"));
  const doneShown = !(session.done === undefined || session.done.since === kept || session.turn.state !== "idle" || session.asked !== undefined || session.dead !== undefined || draft.close === undefined);
  // What waits on the person sits at the foot of the thread in every state, done included, and is not in the ledger. A question or permission whose own
  // card shows it is left out, unless the Session is done and no card is left. A terminal's own permission prompt has no card in the thread, so it stays at the foot.
  const carded = new Set((session.waitingFor ?? []).filter((one) => one.source === "ask_card" || (one.source === "permission" && one.act?.kind !== "message")).map((one) => `waiting${one.id}`));
  const todos = todosOf(session);
  const needs = entries.filter((one) => one.kind === "waiting" && (doneShown || !carded.has(one.key)));
  const ledger = entries.filter((one) => one.kind !== "waiting");
  // What the agent is held on, or asked and was not answered, is the first thing in the Needs you panel on the composer, not the thread's last card.
  const asked: ComponentProps<typeof AskCards>["asked"] =
    session.asked === undefined
      ? undefined
      : {
          command: session.asked.command,
          ...(session.asked.questions === undefined ? {} : { questions: session.asked.questions }),
          ...(session.asked.offers === undefined
            ? {}
            : { offers: (session.asked.reaches === undefined ? helmOfferedOf : reachOfferedOf)(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
          ...(session.asked.reaches === undefined ? {} : { reaches: session.asked.reaches }),
          ...(session.asked.reachesTo === undefined ? {} : { reachesTo: session.asked.reachesTo }),
        };
  const lapsed: ComponentProps<typeof AskCards>["lapsed"] =
    session.lapsed === undefined || session.lapsed.id === answeredLapse
      ? undefined
      : {
          id: session.lapsed.id,
          questions: session.lapsed.questions,
          onAnswer: (answers) => {
            const lapse = session.lapsed!.id;
            void draft.send(session.id, { text: lapsedAnswerText(answers), files: [], sketches: [], tags: [] })?.then((went) => went && setAnsweredLapse(lapse));
          },
          onSkip: () => setAnsweredLapse(session.lapsed!.id),
        };
  const question =
    asked === undefined && lapsed === undefined
      ? undefined
      : {
          id: asked === undefined ? lapsed!.id : (session.asked?.call ?? asked.command),
          text: askText({ ...(asked === undefined ? {} : { asked }), ...(lapsed === undefined ? {} : { lapsed }) }),
          label: askLabel({ ...(asked === undefined ? {} : { asked }), ...(lapsed === undefined ? {} : { lapsed }) }),
          children: <AskCards flat {...(asked === undefined ? {} : { asked, keep: `${session.id}:${session.asked?.call ?? asked.command}` })} onAnswer={(answer, answers, folder) => draft.answer(session.id, answer as SessionAnswer | undefined, answers, folder)} {...(lapsed === undefined ? {} : { lapsed })} />,
        };
  // A judge's concerns wait in the Needs you list too, after the question and before the items.
  const judges = (session.work?.concerns ?? []).map((one) => ({
    id: one.id,
    text: one.text,
    children: <ConcernCard bare concern={one} onSend={() => draft.work?.sendConcern(session.id, one.id)} onDismiss={() => draft.work?.dismissConcern(session.id, one.id)} />,
  }));
  // A pending plan is drawn only while Fleet puts it to him; held, it is not there yet, and a sheet open on it shuts.
  // A plan set aside stays in the thread as a chip, though nothing waits on him for it.
  const planDrawn = session.work?.plan !== undefined && (session.work.plan.approved || session.work.plan.setAside === true || planAsked(session));
  useEffect(() => {
    if (!planDrawn) setReviewing(false);
  }, [planDrawn]);
  const setAside = draft.work?.setAside === undefined ? undefined : () => draft.work?.setAside?.(session.id);
  const approval: PlanApproval | undefined = session.work?.plan === undefined || !planDrawn ? undefined : { ...(session.work.plan.version === undefined ? {} : { version: session.work.plan.version }), ...(session.work.plan.summary === undefined ? {} : { summary: session.work.plan.summary }), groups: session.work.plan.groups.map((one) => ({ id: one.id, agent: one.agent, title: one.title, ...(one.gives === undefined ? {} : { gives: one.gives }), paths: one.paths, tasks: one.tasks.map((task) => ({ id: task.id, text: task.text })) })), approved: session.work.plan.approved, ...(session.work.plan.setAside === true ? { setAside: true as const } : {}), ...(session.pressed?.includes("plan") === true ? { going: true as const } : {}), changes: changeLines(session.work.plan), ...planRevision(session.work.plan), ...(session.work.plan.sketch === undefined ? {} : { sketch: { id: session.work.plan.sketch.id, title: session.work.plan.sketch.title, scene: session.work.plan.sketch.scene } }), ...(session.work.plan.after === undefined ? {} : { after: session.work.plan.after }), onApprove: approvePlan, ...(setAside === undefined ? {} : { onSetAside: setAside }), onReview: () => setReviewing(true) };
  // A plan waiting for approval is a row in Needs you that opens it in full beside the thread, so the panel never stands between him and it.
  const plan = approval === undefined || approval.approved || approval.setAside === true ? undefined : { id: `${approval.version ?? ""}`, onOpen: () => setReviewing(true), ...(setAside === undefined ? {} : { onSetAside: setAside }) };
  const waits = needs.length > 0 || question !== undefined || judges.length > 0 || plan !== undefined;
  // Open only for what the panel lists: a restart is the band's own key, and a band with nothing in it is not drawn open.
  const listed = needs.some((one) => one.restart === undefined) || question !== undefined || judges.length > 0 || plan !== undefined;
  // The delivery question opens the card before he has touched it; once he has closed or opened it, his choice holds.
  const panelOpen = (panel?.id === session.id ? panel.open : holdsDelivery(session)) && waits && listed;
  // The agent's last word is a band of its own on the composer, and Needs you has the slot first: it waits until nothing else does.
  const doneProp: ComponentProps<typeof SessionEndBand>["done"] | undefined = !doneShown
    ? undefined
    : {
        onEnd: () => draft.close?.(session.id),
        made,
        onKeep: () => {
          setKept(session.done?.since);
          // Fleet's `done` item goes too, so the Cockpit and the list stop calling it done.
          if (session.done !== undefined) void dismissWaiting(session.id, `done:${session.done.since}`);
          centre.current?.querySelector<HTMLElement>('[role="textbox"][aria-label="Message"]')?.focus();
        },
      };
  const endShown = doneProp !== undefined && !waits;
  // A hand off that did not start: a band, put away by a press, and the row carries nothing.
  const failedShown = session.handoff === "failed" && failedAway !== session.id;
  // Open on coming into the Session, until he folds it there.
  const endOpen = (endPanel?.id === session.id ? endPanel.open : true) && endShown;
  // The running account of the turn and the page it is building, where the take puts them (`account-place.ts`). Once the turn ends it reads done.
  const place = accountPlace();
  const turnEnded = session.turn.state !== "working";
  const turn: TurnAccount | undefined =
    session.account === undefined
      ? undefined
      : { done: session.account.done, ...(turnEnded || session.account.now === undefined ? {} : { now: session.account.now }), ...(turnEnded || session.account.next === undefined ? {} : { next: session.account.next }) };
  const live: { live?: TurnLive } = session.live === undefined ? {} : { live: { title: session.live.title, view: <PageFrame draft={draft} sessionId={session.id} address={session.live.url} label={session.live.title} onClose={NO_CLOSE} /> } };
  // On the composer, the account yields its slot as the end band does: to Needs you, and to the end and a failed hand off.
  const nowShown = place === "band" && turn !== undefined && !waits && !endShown && !failedShown;
  const nowOpen = nowPanel?.id === session.id && nowPanel.open && nowShown;
  // Where the repository keeps its Armada configuration: the first Job on it says so in words, every Job after in a glyph.
  const root = session.dead !== undefined || session.helm === true || session.terminal === true ? undefined : rootOfSession(session, repositories);
  const profile = root === undefined ? undefined : boundProfile(account, root);
  const stands = root === undefined ? undefined : runBook.placeOf(root, profile);
  const where =
    root === undefined || stands === undefined
      ? undefined
      : {
          place: stands,
          synced: NO_SYNCED,
          profiles: profilesOf(profile),
          onPick: (place: RunsPlace) => runBook.place(root, place),
          onFolder: chooseFolder,
          onDone: () => setFocusFor({ key: session.id, at: Date.now() }),
        };
  // An unread thread cannot say whether anything was sent, and `cardOn` claims the repository's card: neither card nor glyph until it is read.
  const unread = session.transcript === "unread";
  // A repository another Job has run in is not new, whatever this machine remembers: a draft Job there carries no card.
  const inUse = root !== undefined && sessions.some((other) => other.id !== session.id && other.lastTurnAt !== undefined && rootOfSession(other, repositories) === root);
  const cardLead = !unread && where !== undefined && root !== undefined && cardOn(root, session.id, sentTo(session) || inUse);
  const marked = unread || where === undefined || cardLead ? undefined : <RunsPlaceControl {...where} />;
  return (
    <SessionRefs session={session}>
      <ProseTerminal.Provider value={terminal}>
      <SessionWorkspace session={session} ed={planEditing} entries={entries} repositories={repositories} hosts={deck?.hosts} bays={bays} onOpenSketch={(sketch) => read({ kind: "sketch", id: sketch })} onOpenSession={onOpen} onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}>
      <SessionFrame
        state={state}
        said={said}
        id={session.id}
        {...(session.address === undefined ? {} : { address: session.address })}
        {...(session.title === undefined ? {} : { title: session.title })}
        {...(draft.rename === undefined ? {} : { onRename: (title: string) => draft.rename?.(session.id, title) })}
        {...(session.modOutOfDate === true ? { modOutOfDate: true } : {})}
        repository={<SessionRepositoryOf key={session.id} session={session} repositories={repositories} onOpen={onOpen} />}
        {...(fresh === undefined ? {} : { fresh })}
        {...(session.loadout === undefined ? {} : { loadout: { summary: session.loadout, onOpen: () => goes.onOpenLoadout(session.id) } })}
        {...(session.context === undefined ? {} : { context: { ...session.context, ...(session.contextBreakdown === undefined ? {} : { breakdown: session.contextBreakdown }) } })}
        menu={{
          entries: [
            ...(deck?.acts ?? []).map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, icon: one.icon })),
            ...(draft.fork === undefined || session.dead === undefined ? [] : [{ kind: "item", id: "fork", label: "Fork", icon: GitFork } as const]),
            ...(retro.press === undefined ? [] : [{ kind: "item", id: "retro", label: retro.writing.has(session.id) ? "Writing the retro" : "Retro", icon: Rewind, disabled: retro.writing.has(session.id) } as const]),
            ...(draft.close === undefined || session.dead !== undefined ? [] : [{ kind: "separator", id: "end-rule" } as const, { kind: "item", id: "end", label: "End job", icon: PowerOff, danger: true } as const]),
          ],
          onSelect: (id) => {
            if (id === "fork") opening(draft.fork!(session.id), onOpen);
            else if (id === "retro") retro.press?.(session, true);
            else if (id === "end") draft.close?.(session.id);
            else deck?.acts.find((one) => one.id === id)?.run();
          },
        }}
        {...(deck === undefined ? {} : { back: deck.back })}
        {...{
              actions: (
                <>
                  {deck !== undefined ? null : narrow ? (
                    <Tooltip label="Open what this Job holds">
                      <Button variant="ghost" size="sm" aria-label="Attachments" aria-expanded={ledgerOpen} onClick={() => setLedgerOpen(true)}>
                        <PanelRightOpen size={16} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  ) : minimized ? (
                    <Tooltip label="Show attachments">
                      <Button variant="ghost" size="sm" aria-label="Show attachments" onClick={() => minimize(false)}>
                        <PanelRightOpen size={16} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  ) : null}
                  {marked}
                </>
              ),
            }}
        {...(narrow || minimized || deck !== undefined
          ? {}
          : {
              aside: (
                <>
                  <div className="armada-shell__dock-head">
                    <h2 className="armada-shell__dock-title">Ledger</h2>
                    <Tooltip label="Hide attachments">
                      <Button variant="ghost" size="sm" aria-label="Hide attachments" onClick={() => minimize(true)}>
                        <PanelRightClose size={16} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  </div>
                  <SessionLedger key={session.id} entries={ledger} keepKey={session.id} />
                </>
              ),
            })}
      >
        <div ref={centre} className="armada-session-frame__centre">
          <Refused />
          {origin === undefined || carried ? null : (
            <ForkedFrom title={origin.title ?? origin.id} onOpen={() => draft.watch?.(origin.id)}>
              <SessionThread sessionId={origin.id} rows={originRows} onAnswer={() => undefined} onOpenSession={onOpen} />
            </ForkedFrom>
          )}
          {startedBy}
          <RequestOpen.Provider value={openRequest}>
          <SavedOpen.Provider value={openSaved}>
          <SessionThread
            sessionId={session.id}
            rows={shown}
            {...(!cardLead ? {} : { lead: <RunsPlaceCard {...where!} /> })}
            {...(place !== "thread" || turn === undefined || session.account === undefined ? {} : { under: { row: session.account.for, node: <TurnAccountCard key={`turn:${session.id}`} lines={turn} {...live} ended={turnEnded} /> } })}
            {...(session.transcript === undefined ? {} : { transcript: session.transcript })}
            onOpenRetro={openRetro}
            {...(draft.retry === undefined ? {} : { onRetry: (rowId: string) => draft.retry!(session.id, rowId) })}
            {...(draft.forkAt === undefined ? {} : { onForkAt: (pick) => opening(forkAtPick(draft.forkAt!, session.id, pick, setFocusFor), onOpen) })}
            {...(draft.muteNotice === undefined ? {} : { onMuteNotice: (notice, choice) => muteNoticeFor(draft.muteNotice!, session, notice, choice) })}
            {...(session.dead !== undefined ? {} : { onReplyAt: ({ quote }) => {
              quoteIntoDraft(session.id, quote);
              setFocusFor({ key: session.id, at: Date.now() });
            } })}
            {...(approval === undefined ? {} : { approval })}
            onOpenWindow={(url) => draft.openWindow?.(id, url)}
            onOpenSketch={(sketch) => read({ kind: "sketch", id: sketch })}
            onOpenFile={(path) => void openInPanel({ kind: "file", path, from: "session", sessionId: session.id })}
            onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}
            working={session.turn.state === "working"}
            {...(draft.stop === undefined ? {} : { onStop: () => draft.stop!(session.id) })}
            {...(todos.length === 0 || doneProp !== undefined ? {} : { workingOn: { todos } })}
            {...(trailing === undefined ? {} : { trailing })}
            {...(runner === undefined ? {} : { runner })}
            {...(session.terminal === true && draft.command !== undefined ? { onCommand: (text) => draft.command!(session.id, text), commandNames: (session.commands ?? []).map((one) => one.name) } : {})}
            onAnswer={() => undefined}
            onOpenSession={onOpen}
          />
          </SavedOpen.Provider>
          </RequestOpen.Provider>
          {terminal === null ? null : <DockedTerminal dock={terminal.dock} />}
          <CallChip word={callWord(session)} scope={centre} />
          <div className="sv-dock" data-needs={waits || endShown || failedShown || nowShown ? "" : undefined} data-open={panelOpen || endOpen} data-end={endOpen ? "" : undefined} data-now-open={nowOpen ? "" : undefined}>
          <NeedsYouPanel key={session.id} rows={needs} open={panelOpen} onOpen={(open) => setPanel({ id: session.id, open })} {...(question === undefined ? {} : { question })} {...(plan === undefined ? {} : { plan })} judges={judges} />
          {failedShown ? (
            <HandoffBand
              {...(session.handoffReason === undefined ? {} : { reason: session.handoffReason })}
              {...(draft.retryHandoff === undefined ? {} : { onRetry: () => { setFailedAway(undefined); draft.retryHandoff!(session.id); } })}
              {...(dispatchFailure === undefined ? {} : { onDispatch: () => void dispatchFailure(handoffFailure(session)) })}
              onDismiss={() => setFailedAway(session.id)}
            />
          ) : null}
          {endShown && doneProp !== undefined ? <SessionEndBand done={doneProp} open={endOpen} onOpen={(open) => setEndPanel({ id: session.id, open })} /> : null}
          {nowShown ? <TurnAccountBand key={`turn:${session.id}`} lines={turn!} {...live} open={nowOpen} onOpen={(open) => setNowPanel({ id: session.id, open })} /> : null}
          {session.dead !== undefined ? null : (
            <SessionComposer
              modeLocked={session.terminal === true}
              modeHidden={session.terminal === true && session.mode === undefined}
              working={session.turn.state === "working"}
              mode={mode}
              onMode={(next) => draft.tune(session.id, { model: session.model ?? null, effort: session.effort ?? null, mode: next })}
              model={session.model ?? null}
              effort={session.effort ?? null}
              models={draft.models}
              {...(draft.defaultModel === undefined ? {} : { defaultModel: draft.defaultModel })}
              {...(session.resolvedModel === undefined ? {} : { resolvedModel: session.resolvedModel })}
              efforts={draft.efforts}
              onTune={(tuning) => {
                draft.tune(session.id, { ...tuning, mode });
                // A value the proposer picked loses its mark once he changes it.
                if (tuning.model !== (session.model ?? null)) draft.work?.unpick(session.id, "model");
                if (tuning.effort !== (session.effort ?? null)) draft.work?.unpick(session.id, "effort");
              }}
              {...(session.work?.proposal?.picked === undefined ? {} : { proposed: { model: session.work.proposal.picked.includes("model"), effort: session.work.proposal.picked.includes("effort") } })}
              commands={session.terminal === true ? (session.commands ?? []) : hostedCommands(draft.commands)}
              compact={narrow}
              taggable={sessions.filter((one) => one.id !== session.id && one.dead === undefined && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! }))}
              searchFiles={(query) => searchFiles(query, session.manifestId)}
              {...(numbered === undefined ? {} : { numbered })}
              tags={session.pendingTags ?? []}
              draftKey={session.id}
              focusFor={focusFor}
              onTags={(tags) => draft.setTags(session.id, tags)}
              drawn={drawn.map(({ id, title }) => ({ id, title }))}
              onDraw={() => setPadOpen(true)}
              onRemoveDrawn={(id) => setDrawn((was) => was.filter((one) => one.id !== id))}
              onSend={(sent) => {
                if (opensMcp(session, sent.text, sent.files.length)) {
                  goes.onOpenLoadout(session.id, "mcp");
                  return Promise.resolve(true);
                }
                // `! command` runs in the Session's directory, in the docked terminal, and is not a message to the agent.
                const bang = sent.files.length === 0 ? bangOf(sent.text) : undefined;
                if (bang !== undefined && terminal !== null) {
                  terminal.dock.start(bang);
                  return Promise.resolve(true);
                }
                const sketches = drawn;
                const done = draft.send(session.id, { text: sent.text, files: sent.files, sketches, tags: sent.tags as readonly SessionTag[] });
                setDrawn(NO_SKETCHES);
                // The message is in the thread from now, and one that did not go has its own Retry: the box has nothing to take back.
                if (draft.retry !== undefined) return Promise.resolve(true);
                return done?.then((went) => {
                  if (!went) setDrawn((now) => (now.length === 0 ? sketches : now));
                  return went;
                });
              }}
            />
          )}
          </div>
        </div>
        {place === "side" && turn !== undefined ? <TurnAccountPanel key={`turn:${session.id}`} lines={turn} {...live} /> : null}
      </SessionFrame>
      </SessionWorkspace>
      {session.work?.plan === undefined || !planDrawn ? null : (
        <PlanReview open={reviewing} plan={session.work.plan} {...planRevision(session.work.plan)} ed={planEditing!} onApprove={approvePlan} onClose={() => setReviewing(false)} />
      )}
      {deck?.ledger == null || workspace ? null : createPortal(<SessionLedger key={session.id} entries={ledger} keepKey={session.id} />, deck.ledger)}
      {released.dialog}
      {narrow ? (
        <OverTheApp open={ledgerOpen}>
          <Sheet kind="session-ledger" open={ledgerOpen} floating floor={floor} title="Attachments" closeLabel="Close" closeBinding="Esc" onClose={() => setLedgerOpen(false)}>
            <SessionLedger key={session.id} entries={ledger} keepKey={session.id} folded />
          </Sheet>
        </OverTheApp>
      ) : null}
      <ReadingSheet
        sessionId={session.id}
        readSubagent={draft.subagent}
        one={readingOf(session, reading)}
        steps={stepsReadingOf(session, reading, draft)}
        draft={draft}
        onClose={() => setReading(undefined)}
        onOpenLink={goes.onOpenLink}
      />
      <ClashFileSheet session={session} draft={draft} />
      <SketchSheet
        open={padOpen}
        onClose={() => setPadOpen(false)}
        onAttach={(sketch) => {
          setDrawn((was) => [...was, sketch]);
          setPadOpen(false);
        }}
      />
      </ProseTerminal.Provider>
    </SessionRefs>
  );
}

/**
 * A Session's own message box. `onSend` takes the words where the host sends
 * them itself (a call card answers its waiting item with them); with none, the message goes to the Session.
 * `/mcp` in a hosted Job opens its Loadout on its MCP servers, where the host can open one.
 */
export function SessionBox({ session, onOpen, onSend, onOpenLoadout }: { session: Session; onOpen: (id: string) => void; onSend?: (text: string) => void; onOpenLoadout?: LedgerGoes["onOpenLoadout"] | undefined }) {
  const draft = useSessionsDraft();
  const numbered = useNumbered(session.manifestId);
  // A `! command` typed here runs in a pane of this box's own, as the Session page's does.
  const runner = draft === undefined ? undefined : runnerOf(draft, session);
  const dock = useTerminalDock(useCodeRun(runner), session.id);
  if (draft === undefined || session.dead !== undefined) return null;
  const mode: SessionMode = session.mode ?? (session.terminal === true ? "ask" : "auto");
  return (
    <>
      {draft.terminalRuns === undefined ? null : <DockedTerminal dock={dock} />}
      <SessionComposer
        modeLocked={session.terminal === true}
        modeHidden={session.terminal === true && session.mode === undefined}
        working={session.turn.state === "working"}
        mode={mode}
        onMode={(next) => draft.tune(session.id, { model: session.model ?? null, effort: session.effort ?? null, mode: next })}
        model={session.model ?? null}
        effort={session.effort ?? null}
        models={draft.models}
        {...(draft.defaultModel === undefined ? {} : { defaultModel: draft.defaultModel })}
        {...(session.resolvedModel === undefined ? {} : { resolvedModel: session.resolvedModel })}
        efforts={draft.efforts}
        onTune={(tuning) => draft.tune(session.id, { ...tuning, mode })}
        commands={session.terminal === true ? (session.commands ?? []) : hostedCommands(draft.commands)}
        compact
        taggable={[]}
        {...(numbered === undefined ? {} : { numbered })}
        tags={session.pendingTags ?? []}
        draftKey={session.id}
        onTags={(tags) => draft.setTags(session.id, tags)}
        drawn={[]}
        onDraw={() => onOpen(session.id)}
        onRemoveDrawn={() => undefined}
        onSend={(sent) => {
          if (onOpenLoadout !== undefined && opensMcp(session, sent.text, sent.files.length)) {
            onOpenLoadout(session.id, "mcp");
            return Promise.resolve(true);
          }
          if (onSend !== undefined) return onSend(sent.text);
          const bang = sent.files.length === 0 ? bangOf(sent.text) : undefined;
          if (bang !== undefined && draft.terminalRuns !== undefined) {
            dock.start(bang);
            return Promise.resolve(true);
          }
          const done = draft.send(session.id, { text: sent.text, files: sent.files, sketches: [], tags: sent.tags as readonly SessionTag[] });
          // In the thread from now, with a Retry if it did not go: nothing for the box to take back.
          return draft.retry === undefined ? done : Promise.resolve(true);
        }}
      />
    </>
  );
}
