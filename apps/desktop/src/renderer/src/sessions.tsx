// Sessions, as the window draws them: a rail surface, a panel on Overview, and
// who owns a chip anywhere. Apart from `App.tsx`, which is at its length.
// What is drawn is `@armada/components`'; this reads the draft
// (`packages/screens/src/draft/sessions.ts`) into it.

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { ExternalLink, PanelRightClose, PanelRightOpen, PowerOff } from "lucide-react";
import {
  Alert,
  BACKGROUND,
  Button,
  ChipOwnership,
  ContextFigure,
  ForkedFrom,
  OpenInSession,
  PilotAct,
  PilotConfirm,
  PilotExits,
  DockedTerminal,
  ProseTerminal,
  RetroPress,
  Tooltip,
  Sheet,
  SessionComposer,
  SessionFrame,
  SessionLedger,
  SessionList,
  SessionThread,
  SketchPad,
  SketchPreview,
  SketchReading,
  StepsPanel,
  SubagentMarks,
  TileSheet,
  pressedSlot,
  useCodeRun,
  useKept,
  useTerminalDock,
} from "@armada/components";
import type {
  ProseTerminalValue,
  ChipOwnershipValue,
  FreshStart,
  OpenInSessionValue,
  PilotValue,
  LedgerEntry,
  OwnerChipRef,
  OwnerSummary,
  SessionBand,
  SessionGroup,
  SessionRowView,
  SketchBox,
  SketchLine,
  SketchPicture,
  SketchStroke,
  SessionState,
  SessionThreadRow,
  StepsPanelProps,
  RetroRowItem,
} from "@armada/components";
import { sessionOf } from "@armada/jobs";
import { askToOpenRetro } from "./open-retro";
import { handoffOf } from "./session-handoff";
import { readRetro, searchFiles } from "./commands";
import { ArtifactBody, ARTIFACT_GLYPH, isAddress } from "./session-artifact";
import { isUnsentItem, linkEntry, unsentEntry, useStartedBy } from "./session-started";
import { readingOfSketch } from "./session-sketch";
import { forkAtPick, threadRowsOf, upTo } from "./session-thread-rows";
import { backgroundEntryOf } from "./session-background";
import { SubagentThread, useSubagentRead } from "./session-subagent";
import { useMinimized } from "./session-ledger-minimized";
import { helmOfferedOf, reachOfferedOf } from "@armada/screens/src/copy";
import { lapsedAnswerText } from "@armada/screens/src/sessions-wire";
import { attachmentsOf, ownerOf, owesOf, sessionsMatching } from "@armada/screens/src/draft/sessions";
import type { ChipRef, DrawnSketch, PullRequestAct, Session, SessionAnswer, SessionAttachment, SessionMode, SessionsDraft, SessionTag } from "@armada/screens/src/draft/sessions";
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
import type { HeldWorktrees, JobSummary, MergeLines, RepositorySummary, SettingsList } from "@armada/protocol";
import { SessionRefs } from "./helm-refs";
import { pullRequestPlaceOf } from "./pull-request-place";
import { SURFACE, useAtFloor, useNarrow } from "@armada/shell";

import { proposeRequest } from "./dispatch";
import { useBackfilled } from "./backfilled";
import { OPEN_SESSION_ATTRIBUTE } from "./open-session";
import { ARTIFACT_SAID, cardOf, pullRequestCardOf, type Known } from "./session-cards";
import { SessionPullRequest, useSendBack } from "./session-pull-request";
import { useNumbered } from "./numbered";
import { SessionRepositoryOf } from "./session-move";
import { movedRowsOf } from "./session-moved";
import { useSessions, useSessionsDraft, useSessionsSaid } from "./sessions-draft";
import { restartOf } from "./build-change";
import { runnerOf } from "./session-runner";
import { nowOf, todosOf } from "./session-activity";
import { useViewedVersion, useViewing } from "./cockpit/viewed";
import { dismissWaiting } from "./cockpit/dismissed";
import { stateOf } from "./session-state";
import { useLedgerRelease, type LedgerRelease } from "./session-ledger-release";

/** The surfaces the rail and the palette leave off: Sessions, until something serves them. */
export function sessionsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.sessions];
}

type Checks = Extract<SessionAttachment, { kind: "pull_request" }>["checks"];

const checksSaid = (checks: Checks) => {
  if (checks.state === "failed") return `Checks failed: ${checks.failing}`;
  const red = checks.red === undefined ? "" : checks.state === "pending" ? `; failing: ${checks.red}` : `; not required, failing: ${checks.red}`;
  return `${checks.state === "pending" ? "Checks running" : "Checks passed"}${red}`;
};

const redOf = (checks: Checks): { red?: "running" | "advisory" } =>
  checks.state === "failed" || checks.red === undefined ? {} : { red: checks.state === "pending" ? "running" : "advisory" };

const pullRequestsOf = (session: Session) =>
  attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks), ...redOf(one.checks) }));

function summaryOf(session: Session): OwnerSummary {
  return {
    id: session.id,
    ...(session.address === undefined ? {} : { address: session.address }),
    ...(session.title === undefined ? {} : { title: session.title }),
    ...stateOf(session),
    slots: attachmentsOf(session, "slot").map((one) => one.slot),
    pullRequests: pullRequestsOf(session),
    jobs: attachmentsOf(session, "job").filter((one) => one.looking !== true).map((one) => ({ id: one.id, number: one.number })),
    ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
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

/** Who owns each chip, for every chip in the window, and where a Job that went wrong can be talked through. */
export function SessionsOwnership({ onOpen, children }: { onOpen: (sessionId: string) => void; children: ReactNode }) {
  const sessions = useSessions();
  const draft = useSessionsDraft();
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
  const talk = useMemo<OpenInSessionValue | null>(() => {
    if (draft === undefined) return null;
    return {
      targets: sessions.filter((one) => one.title !== undefined && one.terminal !== true && one.dead === undefined).map((one) => ({ id: one.id, title: one.title! })),
      open: (jobId, sessionId) => {
        const tag = draft.taggable().find((one) => one.kind === "job" && one.id === jobId);
        if (tag === undefined) return;
        if (sessionId === undefined) opening(draft.start(tag), onOpen);
        else {
          const had = sessions.find((one) => one.id === sessionId)?.pendingTags ?? [];
          draft.setTags(sessionId, [...had, tag]);
          onOpen(sessionId);
        }
      },
    };
  }, [draft, sessions, onOpen]);
  const [asking, setAsking] = useState<string | null>(null);
  /** Which act Fleet's last refusal answered, so a take over's is said in its confirmation and an exit's under the exits, and neither under the other. */
  const [last, setLast] = useState<"take" | "exit" | null>(null);
  const [starting, setStarting] = useState(false);
  const said = useSessionsSaid();
  const pilot = useMemo<PilotValue | null>(() => {
    // With nothing to take a Job over, no chip offers it.
    if (draft?.pilot === undefined || draft.exit === undefined) return null;
    const { exit } = draft;
    return {
      ask: setAsking,
      pilotedBy: (jobId) => {
        for (const one of sessions) {
          const held = one.attachments.find((a) => a.kind === "job" && a.id === jobId && a.state === "piloted" && a.pilotedElsewhere !== true);
          if (held?.kind === "job") return { id: one.id, title: one.title ?? one.id, number: held.number };
        }
        return undefined;
      },
      exit: (jobId, way) => {
        setLast("exit");
        exit(jobId, way);
      },
      open: onOpen,
      said: last === "exit" ? said : undefined,
    };
  }, [draft, sessions, onOpen, last, said]);
  const asked = asking === null || draft === undefined ? undefined : draft.taggable().find((one) => one.kind === "job" && one.id === asking);
  return (
    <ChipOwnership.Provider value={value}>
      <OpenInSession.Provider value={talk}>
        <PilotAct.Provider value={pilot}>
          {children}
          <PilotConfirm
            open={asked !== undefined}
            title={asked?.title ?? ""}
            said={last === "take" ? said : undefined}
            onCancel={() => setAsking(null)}
            onConfirm={(outcome) => {
              if (asking === null || draft?.pilot === undefined || starting) return;
              setLast("take");
              const started = draft.pilot(asking, outcome);
              // Fleet answers before the confirmation goes: a refusal is said in it, and nothing opens.
              if (typeof started === "string") {
                setAsking(null);
                onOpen(started);
                return;
              }
              setStarting(true);
              void started.then((id) => {
                setStarting(false);
                if (id === undefined) return;
                setAsking(null);
                onOpen(id);
              });
            }}
          />
        </PilotAct.Provider>
      </OpenInSession.Provider>
    </ChipOwnership.Provider>
  );
}

function chipOf(attachment: SessionAttachment): OwnerChipRef | undefined {
  switch (attachment.kind) {
    case "pull_request":
      return { kind: "pull_request", number: attachment.number };
    case "branch":
      return { kind: "branch", name: attachment.name };
    case "slot":
      return { kind: "slot", slot: attachment.slot };
    case "job":
      return { kind: "job", id: attachment.id, number: attachment.number };
    default:
      return undefined;
  }
}

/** The headings Overview's lists use, over the Sessions each holds. A heading with no row is not drawn. */
const HEADINGS: { label: string; has: (state: SessionState) => boolean }[] = [
  { label: "Needs you", has: (state) => state === "waiting" || state === "done" },
  { label: "Running", has: (state) => state === "working" || state === "stuck" || state === "limited" },
  // Failing checks stay here, red: Fleet has told the agent, and it asks through waiting_for if it is blocked.
  // A reply stays here too, with its unread mark until he has opened the Session: it asks nothing of him.
  { label: "Idle", has: (state) => state === "idle" || state === "failing" || state === "replied" },
  { label: "Not started", has: (state) => state === "blank" },
  { label: "Quiet", has: (state) => state === "quiet" },
  { label: "Ended", has: (state) => state === "ended" },
];

/** The views of the list, and the headings each holds. Active on opening. */
const VIEWS = [
  { id: "active", label: "Active", headings: ["Needs you", "Running", "Idle", "Not started"] },
  { id: "quiet", label: "Quiet", headings: ["Quiet"] },
  { id: "ended", label: "Ended", headings: ["Ended"] },
  { id: "all", label: "All", headings: HEADINGS.map((one) => one.label) },
] as const;

type ViewId = (typeof VIEWS)[number]["id"];

const VIEW_KEY = "armada.bridge.sessions-view";

/** Which view the viewer chose. A viewer's convenience, so a blocked store only forgets it. */
function useView(): [ViewId, (next: ViewId) => void] {
  const [view, set] = useState<ViewId>(() => {
    try {
      return VIEWS.find((one) => one.id === localStorage.getItem(VIEW_KEY))?.id ?? "active";
    } catch {
      return "active";
    }
  });
  return [
    view,
    (next) => {
      set(next);
      try {
        localStorage.setItem(VIEW_KEY, next);
      } catch {
        // Not kept: the view is as it was left until the window closes.
      }
    },
  ];
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

/** A repository a band is drawn for, by the Manifest id its Sessions carry. */
export type BandBy = { manifestId: string; name: string };

/** The band for a Session whose repository is not one of those listed, or that has none. */
const OTHER_BAND = "Other";

/** Every Session, searchable, with the act that starts one. On Overview and on the rail surface alike. */
export function SessionsListing({ onOpen, startIn, bandsBy }: { onOpen: (id: string) => void; startIn?: readonly { root: string; name: string }[]; bandsBy?: readonly BandBy[] }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const [query, setQuery] = useState("");
  const [view, setView] = useView();
  const retro = useRetroPress();
  // A reply reads as unread until he has been in its Session, so the rows are read again as he goes.
  useViewedVersion();
  if (draft === undefined) return null;
  const views: SessionRowView[] = sessionsMatching(sessions, query).filter(({ session }) => session.older !== true || query.trim() !== "").map(({ session, matched }) => {
    const { state, said } = stateOf(session);
    const chip = matched === "title" || matched === "id" ? undefined : chipOf(matched);
    const doing = { now: nowOf(session) };
    return {
      id: session.id,
      ...(session.address === undefined ? {} : { address: session.address }),
      ...(session.title === undefined ? {} : { title: session.title }),
      state,
      said,
      slots: attachmentsOf(session, "slot").map((one) => one.slot),
      pullRequests: attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks), ...redOf(one.checks), state: one.state, card: pullRequestCardOf(one) })),
      ...(chip === undefined ? {} : { matched: chip }),
      ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
      ...(session.lastTurnAt === undefined ? {} : { lastTurnAt: session.lastTurnAt }),
      ...(session.modOutOfDate === true ? { modOutOfDate: true } : {}),
      ...(owesOf(session).length > 0 ? { waiting: owesOf(session).map((one) => one.text) } : {}),
      ...(doing.now === undefined ? {} : { now: doing.now }),
      ...(retro.writing.has(session.id) ? { retroWriting: true } : {}),
      ...(session.helm === true ? { helm: true } : {}),
    };
  });
  const shown: readonly string[] = VIEWS.find((one) => one.id === view)!.headings;
  const groupsOf = (rows: readonly SessionRowView[]): SessionGroup[] => HEADINGS.filter((one) => shown.includes(one.label)).map((one) => ({ label: one.label, rows: rows.filter((row) => one.has(row.state)) })).filter((one) => one.rows.length > 0);
  const groups = groupsOf(views);
  // On All, a band for each repository in the listing's order, and a last one for Sessions with no listed repository.
  const repositoryOf = new Map(sessions.map((one) => [one.id, one.manifestId]));
  const known = new Set((bandsBy ?? []).map((one) => one.manifestId));
  const bands: SessionBand[] | undefined =
    bandsBy === undefined
      ? undefined
      : [
          ...bandsBy.map((one) => ({ label: one.name, groups: groupsOf(views.filter((row) => repositoryOf.get(row.id) === one.manifestId)) })),
          { label: OTHER_BAND, groups: groupsOf(views.filter((row) => !known.has(repositoryOf.get(row.id) ?? ""))) },
        ].filter((band) => band.groups.length > 0);
  return (
    <>
      <Refused />
      <SessionList groups={groups} {...(bands === undefined ? {} : { bands })} query={query} onQuery={setQuery} views={VIEWS} view={view} onView={(next) => setView(next as ViewId)} onOpen={onOpen} onStart={(root) => opening(draft.start(undefined, root), onOpen)} {...(startIn === undefined ? {} : { startIn })} {...(retro.press === undefined ? {} : { onRetro: (id: string) => retro.press?.(sessions.find((one) => one.id === id) ?? { id }) })} />
    </>
  );
}

/** Where a ledger row goes: the surface that already shows the thing. */
export type LedgerGoes = {
  onOpenJob: (jobId: string) => void;
  onGoTo: (surfaceId: string) => void;
  onOpenLink: (address: string) => void;
  onOpenCheck: (sessionId: string, run: number) => void;
};

/** What the ledger opens beside it, where no surface of Bridge's draws the thing: a pull request, a sketch, a subagent, an artifact. */
export type Reading = { kind: "pull_request"; number: number } | { kind: "sketch"; id: string } | { kind: "subagent"; id: string } | { kind: "background"; id: string } | { kind: "artifact"; id: string } | { kind: "steps"; id: string };

export const readingOf = (session: Session, open: Reading | undefined): SessionAttachment | undefined =>
  open === undefined || open.kind === "steps"
    ? undefined
    : session.attachments.find((one) =>
        open.kind === "pull_request"
          ? one.kind === "pull_request" && one.number === open.number
          : one.kind === open.kind && "id" in one && one.id === open.id,
      );

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
  lines,
  onClose,
  onOpenLink,
  onAct,
}: {
  sessionId: string;
  readSubagent: SessionsDraft["subagent"];
  one: SessionAttachment | undefined;
  /** An item's steps, read in place of an attachment. */
  steps?: StepsReading;
  draft: SessionsDraft | undefined;
  /** What Fleet serves of the merge lines, for the place a pull request holds in one. */
  lines: MergeLines | null;
  onClose: () => void;
  onOpenLink: (address: string) => void;
  onAct: (number: number, act: PullRequestAct) => Promise<boolean>;
}) {
  const floor = useAtFloor();
  const [pressed, setPressed] = useState<{ number: number; act: PullRequestAct } | undefined>(undefined);
  // A Review Fleet accepted, kept for the Session and pull request it was asked of.
  const [reviewed, setReviewed] = useState<ReadonlySet<string>>(new Set());
  const press = (number: number, act: PullRequestAct) => {
    setPressed({ number, act });
    void onAct(number, act)
      .then((accepted) => {
        if (accepted && act === "review") setReviewed((was) => new Set(was).add(`${sessionId}#${number}`));
      })
      .finally(() => setPressed(undefined));
  };
  const { sentBack, sendBack } = useSendBack(sessionId, draft?.send);
  const subagent = one?.kind === "subagent" ? one : undefined;
  const subagentThread = useSubagentRead(sessionId, subagent, readSubagent);
  if (one?.kind === "pull_request") {
    return (
      <SessionPullRequest
        sessionId={sessionId}
        draft={draft}
        one={one}
        lines={lines}
        floor={floor}
        pending={pressed?.number === one.number ? pressed.act : undefined}
        reviewed={reviewed.has(`${sessionId}#${one.number}`)}
        sentBack={sentBack(one)}
        onAct={(act) => press(one.number, act)}
        onSendBack={() => sendBack(one)}
        onOpenLink={onOpenLink}
        onClose={onClose}
      >
        <Refused />
      </SessionPullRequest>
    );
  }
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
  );
}

type Runner = NonNullable<LedgerEntry["run"]>["runner"];

export function entriesOf(
  session: Session,
  goes: LedgerGoes,
  read: (one: Reading) => void,
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
  const waiting = (session.dead === "ended" ? [] : owesOf(session)).map((one): LedgerEntry => {
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
      ...(act !== undefined || one.steps !== undefined || one.source !== "agent" || answerWaiting === undefined
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
  const drawn = session.attachments.filter((one) => one.kind !== "slot" || !carried.has(one.slot));
  return [
    ...waiting,
    ...drawn.map((one): LedgerEntry => {
      const entry = entryOf(one);
      return { ...entry, ...release?.(one, entry), card: cardOf(one, session, sessions, known) };
    }),
  ];

  function entryOf(one: SessionAttachment): Omit<LedgerEntry, "card"> {
    switch (one.kind) {
      case "forked_to":
      case "forked_from":
      case "started_to":
      case "started_by":
        return linkEntry(one, sessions, open);
      case "slot":
        return {
          key: `slot${one.slot}`,
          kind: "slot",
          name: `Worktree slot ${one.slot}`,
          text: <code>{one.slot}</code>,
          ...(one.handed === undefined ? {} : { handed: `Handed over with Job ${one.handed.job}, not leased` }),
          onOpen: () => slot(one.slot),
        };
      case "branch":
        return {
          key: one.name,
          kind: "branch",
          name: `Branch ${one.name}`,
          text: <code>{one.name}</code>,
          ...(one.handed === undefined ? {} : { handed: `Handed over with Job ${one.handed.job}, not created here` }),
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
      case "job":
        return {
          key: one.id,
          kind: "job",
          name: `Job ${one.number}${one.state === "escalated" ? ", needs you" : one.state === "piloted" ? ", piloted" : ""}`,
          text: (
            <>
              <code>{one.number}</code> {one.title}
            </>
          ),
          ...(one.state === "escalated"
            ? { mark: { glyph: "escalated" as const, said: "Needs you" } }
            : one.state === "running"
              ? { mark: { glyph: "running" as const, said: "Running" } }
              : one.state === "piloted" && one.pilotedElsewhere !== true
                ? { mark: { glyph: "piloted" as const, said: "Piloted: its Drone is stopped and you are working it here" }, exits: <PilotExits jobId={one.id} compact /> }
                : one.attested === true
                  ? { mark: { glyph: "done" as const, said: "Closed by your word, not verified" } }
                  : {}),
          ...(one.looking === true ? { looking: true } : {}),
          ...(one.slot === undefined ? {} : { slot: one.slot }),
          ...(one.state === "landed" || one.state === "superseded" ? { finished: true } : {}),
          onOpen: () => goes.onOpenJob(one.id),
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
          name: `${ARTIFACT_SAID[one.form]} ${one.title}`,
          text: one.title,
          onOpen: () => (one.form === "window" ? openWindow(one.id) : read({ kind: "artifact", id: one.id })),
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

/**
 * What the flight deck changes in a Session it draws on its stage: its own acts beside the Session's, and the ledger drawn where
 * the deck keeps it, in the right column, and not beside the thread. There the ledger is always up, so the fold below the
 * breakpoint and the button that hides it are not drawn.
 */
export type DeckSession = {
  /** The deck's acts, after the ones the Sessions window draws: the pull request, Stop, Open and the way back. */
  acts: ReactNode;
  /** The element the ledger is drawn into. Absent until the deck has drawn it, and the ledger waits for it. */
  ledger: HTMLElement | null;
};

/** One Session open: its conversation in the middle and what it holds at the side, one panel. */
const NO_SKETCHES: DrawnSketch[] = [];
const NO_JOBS: JobSummary[] = [];
const NO_ROWS: SessionThreadRow[] = [];

const NO_REPOSITORIES: readonly RepositorySummary[] = [];

export function SessionView({ session, goes, onOpen, held, board, lines, settings, deck, repositories = NO_REPOSITORIES }: { session: Session; goes: LedgerGoes; onOpen: (id: string) => void; held: HeldReads; board: readonly JobSummary[]; lines: MergeLines | null; settings: SettingsList | null; deck?: DeckSession; repositories?: readonly RepositorySummary[] }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const numbered = useNumbered(session.manifestId);
  const floor = useAtFloor();
  const [reading, setReading] = useState<Reading | undefined>();
  const [slotOpen, setSlotOpen] = useState<number | undefined>();
  const [padOpen, setPadOpen] = useState(false);
  const [drawn, setDrawn] = useKept<DrawnSketch[]>(`session:${session.id}:drawn`, NO_SKETCHES);
  const narrow = useNarrow() && deck === undefined;
  const [ledgerOpen, setLedgerOpen] = useState(false);
  // The Session a fork from a selection opened, whose message box takes focus when it shows.
  const [focusFor, setFocusFor] = useState<string | undefined>();
  // The old Session is writing the handoff a fresh one opens on.
  const [handing, setHanding] = useState(false);
  const askCard = useRef<HTMLDivElement>(null);
  const centre = useRef<HTMLDivElement>(null);
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
  // Opening a pull request's sheet reads it again, so what the sheet says is the forge's now.
  const opened = reading?.kind === "pull_request" ? reading.number : undefined;
  useEffect(() => {
    if (opened !== undefined) refresh?.(id, opened);
  }, [refresh, id, opened]);
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
  const shown = useMemo(() => {
    const { head, tail } = movedRowsOf(session, repositories);
    const before = !carried || origin === undefined ? NO_ROWS : originRows.map((row): SessionThreadRow => ({ ...row, id: `${origin.id}:${row.id}` }));
    return head.length + before.length + tail.length === 0 ? thread : [...head, ...before, ...thread, ...tail];
    // eslint-disable-next-line react-hooks/exhaustive-deps -- originRows is derived from origin and the fork's `at`.
  }, [session.attachments, repositories, thread, carried, origin, forkedFrom?.at]);
  // A run drawn as a terminal: the pane is the view's own, docked above its box.
  const runner = draft === undefined ? undefined : runnerOf(draft, session);
  const dock = useTerminalDock(useCodeRun(runner), session.id);
  if (draft === undefined) return null;
  const slot = held.held.state === "read" ? (held.held.held.slots ?? []).find((one) => one.slot === slotOpen) : undefined;
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
  // `links.openPullRequestsIn` decides where every pull request of the Session opens: Armada's panel, or the forge in the browser.
  const read = (one: Reading) => {
    const address = one.kind === "pull_request" ? attachmentsOf(session, "pull_request").find((pr) => pr.number === one.number)?.address : undefined;
    if (address !== undefined && pullRequestPlaceOf(settings) === "browser") goes.onOpenLink(address);
    else setReading(one);
  };
  const entries = entriesOf(
    released.shown,
    narrow ? { ...goes, onOpenJob: fold(goes.onOpenJob), onGoTo: fold(goes.onGoTo) } : goes,
    narrow ? fold(read) : read,
    narrow ? fold(setSlotOpen) : setSlotOpen,
    sessions,
    narrow ? fold(onOpen) : onOpen,
    (url) => draft.openWindow?.(id, url),
    draft.answerWaiting === undefined ? undefined : (itemId, answer) => draft.answerWaiting?.(id, itemId, answer),
    narrow
      ? () => {
          setLedgerOpen(false);
          askCard.current?.scrollIntoView({ block: "center", behavior: "smooth" });
        }
      : () => askCard.current?.scrollIntoView({ block: "center", behavior: "smooth" }),
    { board, slots: held.held.state === "read" ? (held.held.held.slots ?? []) : [] },
    runner,
    (itemId, reason) => void dismissWaiting(id, itemId, reason),
    released.rowOf,
  );
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
  return (
    <SessionRefs session={session} show={(number) => read({ kind: "pull_request", number })}>
      <ProseTerminal.Provider value={terminal}>
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
        {...(session.context === undefined ? {} : { context: { ...session.context, ...(session.contextBreakdown === undefined ? {} : { breakdown: session.contextBreakdown }) } })}
        {...{
              actions: (
                <>
                  {draft.fork === undefined || session.dead === undefined ? null : (
                    <Tooltip label="Start a new Session with this one's conversation">
                      <Button variant="ghost" size="sm" onClick={() => opening(draft.fork!(session.id), onOpen)}>
                        Fork
                      </Button>
                    </Tooltip>
                  )}
                  {retro.press === undefined ? null : <RetroPress writing={retro.writing.has(session.id)} onPress={() => retro.press?.(session, true)} />}
                  {draft.close === undefined || session.dead !== undefined ? null : (
                    <Tooltip label="End session">
                      <Button variant="ghost" size="sm" aria-label="End session" onClick={() => draft.close?.(session.id)}>
                        <PowerOff size={16} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  )}
                  {deck?.acts}
                  {deck !== undefined ? null : narrow ? (
                    <Tooltip label="Open what this Session holds">
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
          {origin === undefined || carried ? null : (
            <ForkedFrom title={origin.title ?? origin.id} onOpen={() => draft.watch?.(origin.id)}>
              <SessionThread sessionId={origin.id} rows={originRows} onAnswer={() => undefined} onOpenSession={onOpen} />
            </ForkedFrom>
          )}
          {startedBy}
          <SessionThread
            sessionId={session.id}
            rows={shown}
            onOpenRetro={openRetro}
            {...(draft.retry === undefined ? {} : { onRetry: (rowId: string) => draft.retry!(session.id, rowId) })}
            {...(draft.forkAt === undefined ? {} : { onForkAt: (pick) => opening(forkAtPick(draft.forkAt!, session.id, pick, setFocusFor), onOpen) })}
            askRef={askCard}
            onOpenWindow={(url) => draft.openWindow?.(id, url)}
            onOpenSketch={(sketch) => read({ kind: "sketch", id: sketch })}
            onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}
            working={session.turn.state === "working"}
            {...(draft.stop === undefined ? {} : { onStop: () => draft.stop!(session.id) })}
            waiting={needs}
            {...(todos.length === 0 ? {} : { workingOn: { todos } })}
            {...(!doneShown
              ? {}
              : {
                  done: {
                    onEnd: () => draft.close?.(session.id),
                    made,
                    onKeep: () => {
                      setKept(session.done?.since);
                      // Fleet's `done` item goes too, so the Cockpit and the list stop calling it done.
                      if (session.done !== undefined) void dismissWaiting(session.id, `done:${session.done.since}`);
                      centre.current?.querySelector<HTMLElement>('[role="textbox"][aria-label="Message"]')?.focus();
                    },
                  },
                })}
            {...(runner === undefined ? {} : { runner })}
            {...(session.terminal === true && draft.command !== undefined ? { onCommand: (text) => draft.command!(session.id, text), commandNames: (session.commands ?? []).map((one) => one.name) } : {})}
            {...(session.asked === undefined
              ? {}
              : {
                  asked: {
                    command: session.asked.command,
                    ...(session.asked.questions === undefined ? {} : { questions: session.asked.questions }),
                    ...(session.asked.offers === undefined
                      ? {}
                      : { offers: (session.asked.reaches === undefined ? helmOfferedOf : reachOfferedOf)(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
                    ...(session.asked.reaches === undefined ? {} : { reaches: session.asked.reaches }),
                  },
                })}
            onAnswer={(answer, answers, folder) => draft.answer(session.id, answer as SessionAnswer | undefined, answers, folder)}
            {...(session.lapsed === undefined || session.lapsed.id === answeredLapse
              ? {}
              : {
                  lapsed: {
                    id: session.lapsed.id,
                    questions: session.lapsed.questions,
                    onAnswer: (answers) => {
                      const lapse = session.lapsed!.id;
                      void draft.send(session.id, { text: lapsedAnswerText(answers), files: [], sketches: [], tags: [] })?.then((went) => went && setAnsweredLapse(lapse));
                    },
                    onSkip: () => setAnsweredLapse(session.lapsed!.id),
                  },
                })}
            onOpenSession={onOpen}
          />
          {terminal === null ? null : <DockedTerminal dock={terminal.dock} />}
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
              onTune={(tuning) => draft.tune(session.id, { ...tuning, mode })}
              commands={session.terminal === true ? (session.commands ?? []) : draft.commands}
              compact={narrow}
              taggable={[
                ...sessions.filter((one) => one.id !== session.id && one.dead === undefined && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! })),
                ...draft.taggable(),
              ]}
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
      </SessionFrame>
      {deck?.ledger == null ? null : createPortal(<SessionLedger key={session.id} entries={ledger} keepKey={session.id} />, deck.ledger)}
      {released.dialog}
      {narrow ? (
        <Sheet kind="session-ledger" open={ledgerOpen} floating floor={floor} title="Attachments" closeLabel="Close" closeBinding="Esc" onClose={() => setLedgerOpen(false)}>
          <SessionLedger key={session.id} entries={ledger} keepKey={session.id} folded />
        </Sheet>
      ) : null}
      <ReadingSheet
        sessionId={session.id}
        readSubagent={draft.subagent}
        one={readingOf(session, reading)}
        steps={stepsReadingOf(session, reading, draft)}
        draft={draft}
        lines={lines}
        onClose={() => setReading(undefined)}
        onOpenLink={goes.onOpenLink}
        onAct={(number, act) => {
          const done = draft.act(session.id, number, act);
          // Review is a request: the link is the whole of it, and the proposer picks the code review workflow.
          // Where Fleet dispatches it, `act` did, and a second request beside it would review it twice.
          const address = session.attachments.find((one) => one.kind === "pull_request" && one.number === number);
          if (act === "review" && address?.kind === "pull_request" && draft.reviews !== "fleet") void proposeRequest(address.address, []);
          // A merge asked for here settles the Session's own item asking for it, and that tells the agent.
          const asked = (session.waitingFor ?? []).find(
            (one) => one.source === "agent" && one.act?.kind === "approve_pr" && one.act.target.replace(/^#/, "") === String(number),
          );
          if ((act === "merge" || act === "auto_merge") && asked !== undefined)
            void done.then((accepted) => {
              if (accepted) draft.answerWaiting?.(session.id, asked.id, { text: act === "merge" ? `Merged #${number}` : `Auto-merge is on for #${number}` });
            });
          return done;
        }}
      />
      <SketchSheet
        open={padOpen}
        onClose={() => setPadOpen(false)}
        onAttach={(sketch) => {
          setDrawn((was) => [...was, sketch]);
          setPadOpen(false);
        }}
      />
      {slot === undefined ? null : <TileSheet row={{ slot }} floor={floor} onOpenJob={goes.onOpenJob} onClose={() => setSlotOpen(undefined)} />}
      </ProseTerminal.Provider>
    </SessionRefs>
  );
}

/**
 * A Session's own message box, as `SessionMini` draws it. `onSend` takes the words where the host sends
 * them itself (a call card answers its waiting item with them); with none, the message goes to the Session.
 */
export function SessionBox({ session, onOpen, onSend }: { session: Session; onOpen: (id: string) => void; onSend?: (text: string) => void }) {
  const draft = useSessionsDraft();
  const numbered = useNumbered(session.manifestId);
  if (draft === undefined || session.dead !== undefined) return null;
  const mode: SessionMode = session.mode ?? (session.terminal === true ? "ask" : "auto");
  return (
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
      commands={session.terminal === true ? (session.commands ?? []) : draft.commands}
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
        if (onSend !== undefined) return onSend(sent.text);
        const done = draft.send(session.id, { text: sent.text, files: sent.files, sketches: [], tags: sent.tags as readonly SessionTag[] });
        // In the thread from now, with a Retry if it did not go: nothing for the box to take back.
        return draft.retry === undefined ? done : Promise.resolve(true);
      }}
    />
  );
}

/**
 * A Session in miniature, for the Dashboard's pane and a walk window's panel: its thread, what it is
 * held on, and the box to answer it, as its own page draws them. No ledger, no frame; Open goes to the
 * whole Session. `fill`: the thread takes the height it is given rather than the pane's cap.
 */
export function SessionMini({ sessionId, onOpen, fill = false }: { sessionId: string; onOpen: (id: string) => void; fill?: boolean }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const session = sessions.find((one) => one.id === sessionId);
  const watch = draft?.watch;
  const thread = useBackfilled(session === undefined ? NO_ROWS : threadRowsOf(session), sessionId);
  useEffect(() => watch?.(sessionId), [watch, sessionId]);
  if (draft === undefined || session === undefined) return null;
  const runner = runnerOf(draft, session);
  return (
    <div className="armada-session-mini" data-fill={fill ? "" : undefined}>
      <div className="armada-session-mini__thread">
        <SessionThread
          sessionId={session.id}
          rows={thread}
          {...(draft.retry === undefined ? {} : { onRetry: (rowId: string) => draft.retry!(session.id, rowId) })}
          {...(runner === undefined ? {} : { runner })}
          {...(session.terminal === true && draft.command !== undefined ? { onCommand: (text) => draft.command!(session.id, text), commandNames: (session.commands ?? []).map((one) => one.name) } : {})}
          {...(session.asked === undefined
            ? {}
            : {
                asked: {
                  command: session.asked.command,
                  ...(session.asked.questions === undefined ? {} : { questions: session.asked.questions }),
                  ...(session.asked.offers === undefined
                    ? {}
                    : { offers: (session.asked.reaches === undefined ? helmOfferedOf : reachOfferedOf)(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
                  ...(session.asked.reaches === undefined ? {} : { reaches: session.asked.reaches }),
                },
              })}
          onAnswer={(answer, answers, folder) => draft.answer(session.id, answer as SessionAnswer | undefined, answers, folder)}
          onOpenSession={onOpen}
        />
      </div>
      <SessionBox session={session} onOpen={onOpen} />
    </div>
  );
}

/** The rail surface: the list, or the Session open on it. */
/** Sessions in the order the list draws them: under each heading in turn. */
export function listed(sessions: readonly Session[]): string[] {
  // Helm's Session opens the dock and no page, so j and k pass it by.
  return HEADINGS.flatMap((heading) => sessions.filter((one) => heading.has(stateOf(one).state) && one.older !== true && one.helm !== true).map((one) => one.id));
}

/** Whether a key press belongs to a field, which j and k must leave alone. */
const typing = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function SessionsSurface({
  openId,
  onOpen,
  goes,
  held,
  board = NO_JOBS,
  lines = null,
  settings = null,
  startIn,
  bandsBy,
  repositories,
}: {
  openId: string | null;
  onOpen: (id: string) => void;
  goes: LedgerGoes;
  held: HeldReads;
  /** The Board's Jobs, for what a Job row's card says beyond the Session's own record of it. */
  board?: readonly JobSummary[];
  lines?: MergeLines | null;
  settings?: SettingsList | null;
  /** Where New Session may start, on All: it asks which. */
  startIn?: readonly { root: string; name: string }[];
  /** On All with more than one repository: the list is drawn in a band for each, in this order. */
  bandsBy?: readonly BandBy[];
  /** Every repository Fleet serves, for the one a Session is in and where it can be moved. */
  repositories?: readonly RepositorySummary[];
}) {
  const sessions = useSessions();
  const open = sessions.find((one) => one.id === openId);
  useViewing(open?.id);
  // j goes to the next Session down the list and k to the one above, opening each. Local to this page.
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      // `move_focus`' first two slots, down and up, as this person has them.
      const slot = pressedSlot("move_focus", event);
      if ((slot !== 0 && slot !== 1) || event.defaultPrevented || typing(event.target)) return;
      const ids = listed(sessions);
      const at = openId === null ? -1 : ids.indexOf(openId);
      const next = ids[slot === 0 ? at + 1 : at - 1];
      if (next === undefined || (at === -1 && slot === 1)) return;
      event.preventDefault();
      onOpen(next);
    };
    window.addEventListener("keydown", press);
    return () => window.removeEventListener("keydown", press);
  }, [sessions, openId, onOpen]);
  return (
    <div className="armada-screen__overview" {...(open === undefined ? {} : { [OPEN_SESSION_ATTRIBUTE]: open.id })}>
      {open === undefined ? <SessionsListing onOpen={onOpen} {...(startIn === undefined ? {} : { startIn })} {...(bandsBy === undefined ? {} : { bandsBy })} /> : <SessionView session={open} goes={goes} onOpen={onOpen} held={held} board={board} lines={lines} settings={settings} {...(repositories === undefined ? {} : { repositories })} />}
    </div>
  );
}
