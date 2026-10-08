// Sessions, as the window draws them: a rail surface, a panel on Overview, and
// who owns a chip anywhere. Apart from `App.tsx`, which is at its length.
// What is drawn is `@armada/components`'; this reads the draft
// (`packages/screens/src/draft/sessions.ts`) into it.

import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { ExternalLink, GitMerge, PanelRightClose, PanelRightOpen } from "lucide-react";
import {
  Alert,
  Button,
  ChipOwnership,
  ForkedFrom,
  OpenInSession,
  PilotAct,
  PilotConfirm,
  PilotExits,
  Tooltip,
  Prose,
  PullRequestActs,
  PullRequestCard,
  Sheet,
  SessionComposer,
  SessionFrame,
  SessionLedger,
  SessionList,
  SessionThread,
  SketchPad,
  SketchPreview,
  TileSheet,
  useKept,
} from "@armada/components";
import type {
  ChipOwnershipValue,
  OpenInSessionValue,
  PilotValue,
  LedgerEntry,
  OwnerChipRef,
  OwnerSummary,
  SessionGroup,
  SessionRowView,
  SketchBox,
  SketchLine,
  SketchPicture,
  SketchStroke,
  SessionState,
  SessionThreadRow,
} from "@armada/components";
import { ArtifactBody, ARTIFACT_GLYPH, isAddress } from "./session-artifact";
import { helmOfferedOf } from "@armada/screens/src/copy";
import { attachmentsOf, isBlank, ownerOf, sessionsMatching } from "@armada/screens/src/draft/sessions";
import type { ChipRef, DrawnSketch, PullRequestAct, Session, SessionAnswer, SessionAttachment, SessionMode, SessionsDraft, SessionTag, SubagentThread as Subagent } from "@armada/screens/src/draft/sessions";
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
import type { HeldWorktrees } from "@armada/protocol";
import { SURFACE, useAtFloor, useNarrow } from "@armada/shell";

import { proposeRequest } from "./dispatch";
import { useSessions, useSessionsDraft, useSessionsSaid } from "./sessions-draft";

/** The surfaces the rail and the palette leave off: Sessions, until something serves them. */
export function sessionsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.sessions];
}

function stateOf(session: Session): { state: SessionState; said: string } {
  if (session.dead === "ended") return { state: "ended", said: "Ended" };
  if (session.dead === "quiet") return { state: "quiet", said: "Not reachable" };
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return { state: "working", said: by === undefined ? "Working" : `Woken by ${by.title}` };
  }
  if (isBlank(session)) return { state: "blank", said: "Blank: no slot, no branch" };
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.checks.state === "failed");
  if (failing !== undefined) return { state: "failing", said: `Checks failed on #${failing.number}` };
  if (session.asked !== undefined) return { state: "waiting", said: "Waiting on you" };
  return { state: "idle", said: "Idle" };
}

type Checks = Extract<SessionAttachment, { kind: "pull_request" }>["checks"];

const checksSaid = (checks: Checks) =>
  checks.state === "failed" ? `Checks failed: ${checks.failing}` : checks.state === "pending" ? "Checks running" : "Checks passed";

const pullRequestsOf = (session: Session) =>
  attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks) }));

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
  { label: "Needs you", has: (state) => state === "waiting" || state === "failing" },
  { label: "Running", has: (state) => state === "working" },
  { label: "Idle", has: (state) => state === "idle" },
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

/** Every Session, searchable, with the act that starts one. On Overview and on the rail surface alike. */
export function SessionsListing({ onOpen }: { onOpen: (id: string) => void }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const [query, setQuery] = useState("");
  const [view, setView] = useView();
  if (draft === undefined) return null;
  const views: SessionRowView[] = sessionsMatching(sessions, query).filter(({ session }) => session.older !== true || query.trim() !== "").map(({ session, matched }) => {
    const { state, said } = stateOf(session);
    const chip = matched === "title" || matched === "id" ? undefined : chipOf(matched);
    return {
      id: session.id,
      ...(session.address === undefined ? {} : { address: session.address }),
      ...(session.title === undefined ? {} : { title: session.title }),
      state,
      said,
      slots: attachmentsOf(session, "slot").map((one) => one.slot),
      pullRequests: attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks), state: one.state })),
      ...(chip === undefined ? {} : { matched: chip }),
      ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
      ...(session.lastTurnAt === undefined ? {} : { lastTurnAt: session.lastTurnAt }),
      ...(session.modOutOfDate === true ? { modOutOfDate: true } : {}),
    };
  });
  const shown: readonly string[] = VIEWS.find((one) => one.id === view)!.headings;
  const groups: SessionGroup[] = HEADINGS.filter((one) => shown.includes(one.label)).map((one) => ({ label: one.label, rows: views.filter((row) => one.has(row.state)) })).filter((one) => one.rows.length > 0);
  return (
    <>
      <Refused />
      <SessionList groups={groups} query={query} onQuery={setQuery} views={VIEWS} view={view} onView={(next) => setView(next as ViewId)} onOpen={onOpen} onStart={() => opening(draft.start(), onOpen)} />
    </>
  );
}

function threadRowsOf(session: Session): SessionThreadRow[] {
  return threadRowsFrom(session.rows);
}

function threadRowsFrom(rows: Session["rows"]): SessionThreadRow[] {
  return rows.map((row): SessionThreadRow => {
    if (row.kind === "lease" || row.kind === "tool" || row.kind === "handoff" || row.kind === "command" || row.kind === "compaction" || row.kind === "window" || row.kind === "check") return row;
    if (row.from.kind === "session") {
      return { id: row.id, at: row.at, kind: "message", from: "session", sender: { id: row.from.id, title: row.from.title }, text: row.text };
    }
    return {
      id: row.id,
      at: row.at,
      kind: "message",
      from: row.from.kind,
      text: row.text,
      ...(row.files === undefined ? {} : { files: row.files }),
      ...(row.sketches === undefined ? {} : { sketches: row.sketches }),
      ...(row.tags === undefined ? {} : { tags: row.tags }),
    };
  });
}

/** Where a ledger row goes: the surface that already shows the thing. */
export type LedgerGoes = {
  onOpenJob: (jobId: string) => void;
  onGoTo: (surfaceId: string) => void;
  onOpenLink: (address: string) => void;
  onOpenCheck: (sessionId: string, run: number) => void;
};

/** What the ledger opens beside it, where no surface of Bridge's draws the thing: a pull request, a sketch, a subagent, an artifact. */
type Reading = { kind: "pull_request"; number: number } | { kind: "sketch"; id: string } | { kind: "subagent"; id: string } | { kind: "artifact"; id: string };

const readingOf = (session: Session, open: Reading | undefined): SessionAttachment | undefined =>
  open === undefined
    ? undefined
    : session.attachments.find((one) =>
        open.kind === "pull_request"
          ? one.kind === "pull_request" && one.number === open.number
          : one.kind === open.kind && "id" in one && one.id === open.id,
      );

/** What is true of a pull request beyond its Checks, as bare facts. */
const factsOf = (one: Extract<SessionAttachment, { kind: "pull_request" }>): string[] =>
  [one.state === "draft" ? "Draft" : undefined, one.auto && one.state !== "merged" ? "Auto-merge on" : undefined].filter(
    (fact): fact is string => fact !== undefined,
  );

/** How often a running subagent's transcript is read again. */
const SUBAGENT_EVERY_MS = 1500;

/**
 * A subagent's own thread, drawn as the Session's is. Read again while it runs; the report it ends
 * on is the last message of the thread, and stands alone only where the thread could not be read.
 */
function SubagentThread({
  sessionId,
  one,
  read,
}: {
  sessionId: string;
  one: Extract<SessionAttachment, { kind: "subagent" }>;
  read: SessionsDraft["subagent"];
}) {
  const [thread, setThread] = useState<Subagent | undefined>(undefined);
  const running = one.state === "running";
  useEffect(() => {
    setThread(undefined);
    if (read === undefined) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const look = async () => {
      const got = await read(sessionId, one.id);
      if (!live) return;
      if (got !== undefined) setThread(got);
      if (running && got?.finished !== true) timer = setTimeout(() => void look(), SUBAGENT_EVERY_MS);
    };
    void look();
    return () => {
      live = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [read, sessionId, one.id, running]);
  const rows = thread === undefined ? [] : threadRowsFrom(thread.rows);
  const report = one.report ?? thread?.report;
  if (rows.length === 0) return report === undefined ? null : <Prose text={report} />;
  return (
    <div className="armada-session-reading" data-thread>
      <SessionThread sessionId={`${sessionId}/${one.id}`} rows={rows} onAnswer={() => undefined} onOpenSession={() => undefined} />
    </div>
  );
}

function ReadingSheet({
  sessionId,
  readSubagent,
  one,
  draft,
  onClose,
  onOpenLink,
  onAct,
}: {
  sessionId: string;
  readSubagent: SessionsDraft["subagent"];
  one: SessionAttachment | undefined;
  draft: SessionsDraft | undefined;
  onClose: () => void;
  onOpenLink: (address: string) => void;
  onAct: (number: number, act: PullRequestAct) => void;
}) {
  const floor = useAtFloor();
  const title =
    one === undefined
      ? ""
      : one.kind === "pull_request"
        ? `Pull request #${one.number}`
        : one.kind === "sketch"
          ? `Sketch ${one.title}`
          : one.kind === "subagent"
            ? `Subagent ${one.task}`
            : one.kind === "artifact"
              ? one.title
              : "";
  const artifact = one?.kind === "artifact" ? one : undefined;
  const Glyph = artifact === undefined ? undefined : ARTIFACT_GLYPH[artifact.form];
  return (
    <Sheet
      kind="session-reading"
      open={one !== undefined && title !== ""}
      floating
      floor={floor}
      title={title}
      bleed={artifact !== undefined || one?.kind === "subagent"}
      {...(Glyph === undefined ? {} : { leading: <Glyph size={16} strokeWidth={2} aria-hidden /> })}
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
      {one?.kind === "pull_request" ? (
        <div className="armada-session-reading">
          <PullRequestCard
            number={`#${one.number}`}
            address={one.address}
            title={one.title}
            branch={one.branch}
            checks={checksSaid(one.checks)}
            {...(one.state === "merged" ? { state: { status: "completed-success", icon: GitMerge, label: "Merged" } } : {})}
            onOpen={() => onOpenLink(one.address)}
          >
            {factsOf(one).map((fact) => (
              <span key={fact}>{fact}</span>
            ))}
          </PullRequestCard>
          <PullRequestActs state={one.state} checks={one.checks.state} auto={one.auto} onAct={(act) => onAct(one.number, act)} />
        </div>
      ) : one?.kind === "sketch" ? (
        <SketchPreview label={one.title} boxes={one.drawing.boxes} lines={one.drawing.lines} strokes={one.drawing.strokes ?? []} pictures={[]} />
      ) : one?.kind === "subagent" ? (
        <SubagentThread sessionId={sessionId} one={one} read={readSubagent} />
      ) : one?.kind === "artifact" ? (
        <ArtifactBody draft={draft} sessionId={sessionId} form={one.form} id={one.id} title={one.title} onClose={onClose} />
      ) : null}
    </Sheet>
  );
}

const BLANK: Drawing = { shapes: [], joins: [], strokes: [], pictures: [] };

/** The pad Dispatch draws on, in a sheet over the Session: what is drawn goes with the next message. */
function SketchSheet({ open, onClose, onAttach }: { open: boolean; onClose: () => void; onAttach: (sketch: DrawnSketch) => void }) {
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

const ARTIFACT_SAID = { page: "Published page", file: "File written", image: "Looked at", doc: "Doc", window: "Shown in a window" } as const;

function entriesOf(
  session: Session,
  goes: LedgerGoes,
  read: (one: Reading) => void,
  slot: (n: number) => void,
  sessions: readonly Session[],
  open: (id: string) => void,
  openWindow: (url: string) => void,
): LedgerEntry[] {
  return session.attachments.map((one): LedgerEntry => {
    switch (one.kind) {
      case "forked_to":
      case "forked_from": {
        const title = sessions.find((other) => other.id === one.id)?.title ?? one.id;
        const way = one.kind === "forked_to" ? "Forked to" : "Forked from";
        return { key: `${one.kind}${one.id}`, kind: "fork", name: `${way} ${title}`, text: `${way} ${title}`, onOpen: () => open(one.id) };
      }
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
  });
}

/** What Cleanup holds, read here for a slot's panel. */
export type HeldReads = { held: HeldWorktrees; onWant: (want: boolean) => void };

/** One Session open: its conversation in the middle and what it holds at the side, one panel. */
/** Whether the ledger is hidden beside the thread, kept for the window. A viewer's convenience, so a blocked store only forgets it. */
function useMinimized(): [boolean, (minimized: boolean) => void] {
  const [minimized, set] = useState(() => {
    try {
      return localStorage.getItem("armada.session-ledger.minimized") === "1";
    } catch {
      return false;
    }
  });
  return [
    minimized,
    (next) => {
      set(next);
      try {
        localStorage.setItem("armada.session-ledger.minimized", next ? "1" : "0");
      } catch {
        // Not kept: the ledger is as it was left until the window closes.
      }
    },
  ];
}

const NO_SKETCHES: DrawnSketch[] = [];

function SessionView({ session, goes, onOpen, held }: { session: Session; goes: LedgerGoes; onOpen: (id: string) => void; held: HeldReads }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const floor = useAtFloor();
  const [reading, setReading] = useState<Reading | undefined>();
  const [slotOpen, setSlotOpen] = useState<number | undefined>();
  const [padOpen, setPadOpen] = useState(false);
  const [drawn, setDrawn] = useKept<DrawnSketch[]>(`session:${session.id}:drawn`, NO_SKETCHES);
  const narrow = useNarrow();
  const [ledgerOpen, setLedgerOpen] = useState(false);
  const [minimized, minimize] = useMinimized();
  const { onWant } = held;
  const { id } = session;
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
  if (draft === undefined) return null;
  const slot = held.held.state === "read" ? (held.held.held.slots ?? []).find((one) => one.slot === slotOpen) : undefined;
  const mode: SessionMode = session.mode ?? (session.terminal === true ? "ask" : "auto");
  // Folded below the breakpoint, a press on a row closes the ledger's sheet first, so what it opens is not drawn over it.
  const fold = <T extends unknown[]>(open: (...args: T) => void) => (...args: T) => {
    setLedgerOpen(false);
    open(...args);
  };
  const entries = entriesOf(
    session,
    narrow ? { ...goes, onOpenJob: fold(goes.onOpenJob), onGoTo: fold(goes.onGoTo) } : goes,
    narrow ? fold(setReading) : setReading,
    narrow ? fold(setSlotOpen) : setSlotOpen,
    sessions,
    narrow ? fold(onOpen) : onOpen,
    (url) => draft.openWindow?.(id, url),
  );
  return (
    <>
      <SessionFrame
        state={state}
        said={said}
        id={session.id}
        {...(session.address === undefined ? {} : { address: session.address })}
        {...(session.title === undefined ? {} : { title: session.title })}
        {...(draft.rename === undefined ? {} : { onRename: (title: string) => draft.rename?.(session.id, title) })}
        {...(session.modOutOfDate === true ? { modOutOfDate: true } : {})}
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
                  {draft.close === undefined || session.terminal === true || session.dead !== undefined ? null : (
                    <Tooltip label="End this Session and park its slot">
                      <Button variant="ghost" size="sm" onClick={() => draft.close?.(session.id)}>
                        Close
                      </Button>
                    </Tooltip>
                  )}
                  {narrow ? (
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
        {...(narrow || minimized
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
                  <SessionLedger entries={entries} />
                </>
              ),
            })}
      >
        <div className="armada-session-frame__centre">
          {origin === undefined ? null : (
            <ForkedFrom title={origin.title ?? origin.id} onOpen={() => draft.watch?.(origin.id)}>
              <SessionThread sessionId={origin.id} rows={threadRowsOf(origin)} onAnswer={() => undefined} onOpenSession={onOpen} />
            </ForkedFrom>
          )}
          <SessionThread
            sessionId={session.id}
            rows={threadRowsOf(session)}
            onOpenWindow={(url) => draft.openWindow?.(id, url)}
            onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}
            working={session.turn.state === "working"}
            {...(session.asked === undefined
              ? {}
              : {
                  asked: {
                    command: session.asked.command,
                    ...(session.asked.questions === undefined ? {} : { questions: session.asked.questions }),
                    ...(session.asked.offers === undefined
                      ? {}
                      : { offers: helmOfferedOf(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
                  },
                })}
            onAnswer={(answer, answers) => draft.answer(session.id, answer as SessionAnswer | undefined, answers)}
            onOpenSession={onOpen}
          />
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
              efforts={draft.efforts}
              onTune={(tuning) => draft.tune(session.id, { ...tuning, mode })}
              commands={session.terminal === true ? (session.commands ?? []) : draft.commands}
              compact={narrow}
              taggable={[
                ...sessions.filter((one) => one.id !== session.id && one.dead === undefined && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! })),
                ...draft.taggable(),
              ]}
              tags={session.pendingTags ?? []}
              draftKey={session.id}
              onTags={(tags) => draft.setTags(session.id, tags)}
              drawn={drawn.map(({ id, title }) => ({ id, title }))}
              onDraw={() => setPadOpen(true)}
              onRemoveDrawn={(id) => setDrawn((was) => was.filter((one) => one.id !== id))}
              onSend={(sent) => {
                draft.send(session.id, { text: sent.text, files: sent.files, sketches: drawn, tags: sent.tags as readonly SessionTag[] });
                setDrawn(NO_SKETCHES);
              }}
            />
          )}
        </div>
      </SessionFrame>
      {narrow ? (
        <Sheet kind="session-ledger" open={ledgerOpen} floating floor={floor} title="Attachments" closeLabel="Close" closeBinding="Esc" onClose={() => setLedgerOpen(false)}>
          <SessionLedger entries={entries} folded />
        </Sheet>
      ) : null}
      <ReadingSheet
        sessionId={session.id}
        readSubagent={draft.subagent}
        one={readingOf(session, reading)}
        draft={draft}
        onClose={() => setReading(undefined)}
        onOpenLink={goes.onOpenLink}
        onAct={(number, act) => {
          draft.act(session.id, number, act);
          // Review is a request: the link is the whole of it, and the proposer picks the code review workflow.
          // Where Fleet dispatches it, `act` did, and a second request beside it would review it twice.
          const address = session.attachments.find((one) => one.kind === "pull_request" && one.number === number);
          if (act === "review" && address?.kind === "pull_request" && draft.reviews !== "fleet") void proposeRequest(address.address, []);
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
    </>
  );
}

/** The rail surface: the list, or the Session open on it. */
/** Sessions in the order the list draws them: under each heading in turn. */
export function listed(sessions: readonly Session[]): string[] {
  return HEADINGS.flatMap((heading) => sessions.filter((one) => heading.has(stateOf(one).state) && one.older !== true).map((one) => one.id));
}

/** Whether a key press belongs to a field, which j and k must leave alone. */
const typing = (target: EventTarget | null): boolean =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function SessionsSurface({ openId, onOpen, goes, held }: { openId: string | null; onOpen: (id: string) => void; goes: LedgerGoes; held: HeldReads }) {
  const sessions = useSessions();
  const open = sessions.find((one) => one.id === openId);
  // j goes to the next Session down the list and k to the one above, opening each. Local to this page.
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if ((event.key !== "j" && event.key !== "k") || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      const ids = listed(sessions);
      const at = openId === null ? -1 : ids.indexOf(openId);
      const next = ids[event.key === "j" ? at + 1 : at - 1];
      if (next === undefined || (at === -1 && event.key === "k")) return;
      event.preventDefault();
      onOpen(next);
    };
    window.addEventListener("keydown", press);
    return () => window.removeEventListener("keydown", press);
  }, [sessions, openId, onOpen]);
  return (
    <div className="armada-screen__overview">
      {open === undefined ? <SessionsListing onOpen={onOpen} /> : <SessionView session={open} goes={goes} onOpen={onOpen} held={held} />}
    </div>
  );
}
