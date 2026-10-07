// Sessions, as the window draws them: a rail surface, a panel on Overview, and
// who owns a chip anywhere. Apart from `App.tsx`, which is at its length.
// What is drawn is `@armada/components`'; this reads the draft
// (`packages/screens/src/draft/sessions.ts`) into it.

import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  ChipOwnership,
  Prose,
  PullRequestCard,
  Sheet,
  SessionComposer,
  SessionFrame,
  SessionLedger,
  SessionList,
  SessionThread,
  SketchPreview,
} from "@armada/components";
import type {
  ChipOwnershipValue,
  LedgerEntry,
  OwnerChipRef,
  OwnerSummary,
  SessionGroup,
  SessionRowView,
  SessionState,
  SessionThreadRow,
} from "@armada/components";
import { attachmentsOf, isBlank, ownerOf, sessionsMatching } from "@armada/screens/src/draft/sessions";
import type { ChipRef, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import { SURFACE, useAtFloor } from "@armada/shell";

import { useSessions, useSessionsDraft } from "./sessions-draft";

/** The surfaces the rail and the palette leave off: Sessions, until something serves them. */
export function sessionsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.sessions];
}

function stateOf(session: Session): { state: SessionState; said: string } {
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return { state: "working", said: by === undefined ? "Working" : `Woken by ${by.title}` };
  }
  if (isBlank(session)) return { state: "blank", said: "Blank: no slot, no branch" };
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.checks.state === "failed");
  if (failing !== undefined) return { state: "failing", said: `Checks failed on #${failing.number}` };
  return { state: "waiting", said: "Waiting on you" };
}

type Checks = Extract<SessionAttachment, { kind: "pull_request" }>["checks"];

const checksSaid = (checks: Checks) =>
  checks.state === "failed" ? `Checks failed: ${checks.failing}` : checks.state === "pending" ? "Checks running" : "Checks passed";

const pullRequestsOf = (session: Session) =>
  attachmentsOf(session, "pull_request").map((one) => ({ number: one.number, checks: one.checks.state, said: checksSaid(one.checks) }));

function summaryOf(session: Session): OwnerSummary {
  return {
    id: session.id,
    ...(session.title === undefined ? {} : { title: session.title }),
    ...stateOf(session),
    slots: attachmentsOf(session, "slot").map((one) => one.slot),
    pullRequests: pullRequestsOf(session),
    jobs: attachmentsOf(session, "job").map((one) => ({ id: one.id, number: one.number })),
    ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
  };
}

/** Who owns each chip, for every chip in the window; opening an owner is the host's. */
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
  { label: "Not started", has: (state) => state === "blank" },
];

/** Every Session, searchable, with the act that starts one. On Overview and on the rail surface alike. */
export function SessionsListing({ onOpen }: { onOpen: (id: string) => void }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const [query, setQuery] = useState("");
  if (draft === undefined) return null;
  const views: SessionRowView[] = sessionsMatching(sessions, query).map(({ session, matched }) => {
    const { state, said } = stateOf(session);
    const chip = matched === "title" || matched === "id" ? undefined : chipOf(matched);
    return {
      id: session.id,
      ...(session.title === undefined ? {} : { title: session.title }),
      state,
      said,
      slots: attachmentsOf(session, "slot").map((one) => one.slot),
      pullRequests: pullRequestsOf(session),
      jobs: attachmentsOf(session, "job").map((one) => ({ id: one.id, number: one.number })),
      ...(chip === undefined ? {} : { matched: chip }),
      ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
    };
  });
  const groups: SessionGroup[] = HEADINGS.map((one) => ({ label: one.label, rows: views.filter((row) => one.has(row.state)) })).filter((one) => one.rows.length > 0);
  return <SessionList groups={groups} query={query} onQuery={setQuery} onOpen={onOpen} onStart={() => onOpen(draft.start())} />;
}

function threadRowsOf(session: Session): SessionThreadRow[] {
  return session.rows.map((row): SessionThreadRow => {
    if (row.kind === "lease" || row.kind === "tool") return row;
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
      ...(row.mentions === undefined ? {} : { mentions: row.mentions }),
    };
  });
}

/** Where a ledger row goes: the surface that already shows the thing. */
export type LedgerGoes = {
  onOpenJob: (jobId: string) => void;
  onGoTo: (surfaceId: string) => void;
  onOpenLink: (address: string) => void;
};

/** What the ledger opens beside it, where no surface of Bridge's draws the thing: a pull request, a sketch, a subagent. */
function Reading({ one, onClose, onOpenLink }: { one: SessionAttachment | undefined; onClose: () => void; onOpenLink: (address: string) => void }) {
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
            : "";
  return (
    <Sheet kind="session-reading" open={one !== undefined && title !== ""} floating floor={floor} title={title} closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      {one?.kind === "pull_request" ? (
        <PullRequestCard
          number={`#${one.number}`}
          address={one.address}
          title={one.title}
          branch={one.branch}
          checks={checksSaid(one.checks)}
          onOpen={() => onOpenLink(one.address)}
        />
      ) : one?.kind === "sketch" ? (
        <SketchPreview label={one.title} boxes={one.drawing.boxes} lines={one.drawing.lines} strokes={[]} pictures={[]} />
      ) : one?.kind === "subagent" ? (
        one.report === undefined ? null : <Prose text={one.report} />
      ) : null}
    </Sheet>
  );
}

function entriesOf(session: Session, goes: LedgerGoes, read: (one: SessionAttachment) => void): LedgerEntry[] {
  return session.attachments.map((one): LedgerEntry => {
    switch (one.kind) {
      case "slot":
        return { key: `slot${one.slot}`, kind: "slot", name: `Worktree slot ${one.slot}`, text: <code>{one.slot}</code>, onOpen: () => goes.onGoTo(SURFACE.worktrees) };
      case "branch":
        return { key: one.name, kind: "branch", name: `Branch ${one.name}`, text: <code>{one.name}</code>, onOpen: () => goes.onGoTo(SURFACE.worktrees) };
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
          mark: { glyph: one.checks.state, said: checksSaid(one.checks) },
          onOpen: () => read(one),
        };
      case "job":
        return {
          key: one.id,
          kind: "job",
          name: `Job ${one.number}`,
          text: (
            <>
              <code>{one.number}</code> {one.title}
            </>
          ),
          slot: one.slot,
          onOpen: () => goes.onOpenJob(one.id),
        };
      case "studio":
        return { key: one.id, kind: "studio", name: `Studio ${one.title}`, text: one.title, onOpen: () => goes.onGoTo(SURFACE.studios) };
      case "sketch":
        return { key: one.id, kind: "sketch", name: `Sketch ${one.title}`, text: one.title, onOpen: () => read(one) };
      case "subagent":
        return {
          key: one.id,
          kind: "subagent",
          name: `Subagent ${one.task}, ${one.state}`,
          text: one.task,
          mark: { glyph: one.state, said: one.state === "running" ? "Running" : "Done" },
          onOpen: () => read(one),
        };
    }
  });
}

/** One Session open: its conversation in the middle and what it holds at the side, one panel. */
function SessionView({ session, goes, onOpen }: { session: Session; goes: LedgerGoes; onOpen: (id: string) => void }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const [reading, setReading] = useState<SessionAttachment | undefined>();
  const { state, said } = stateOf(session);
  if (draft === undefined) return null;
  return (
    <>
      <SessionFrame state={state} said={said} id={session.id} {...(session.title === undefined ? {} : { title: session.title })}>
        <div className="armada-session-frame__centre">
          <SessionThread
            rows={threadRowsOf(session)}
            {...(session.asked === undefined ? {} : { asked: session.asked })}
            onAnswer={() => draft.answer(session.id)}
            onOpenSession={onOpen}
          />
          <SessionComposer
            working={session.turn.state === "working"}
            model={session.model ?? null}
            effort={session.effort ?? null}
            models={draft.models}
            efforts={draft.efforts}
            onTune={(tuning) => draft.tune(session.id, tuning)}
            commands={draft.commands}
            sessions={sessions.filter((one) => one.id !== session.id && one.title !== undefined).map((one) => ({ id: one.id, title: one.title! }))}
            sketches={draft.sketches}
            onSend={(sent) =>
              draft.send(session.id, { text: sent.text, files: sent.files, sketches: sent.sketches, mentions: sent.mentions })
            }
          />
        </div>
        <SessionLedger entries={entriesOf(session, goes, setReading)} />
      </SessionFrame>
      <Reading one={reading} onClose={() => setReading(undefined)} onOpenLink={goes.onOpenLink} />
    </>
  );
}

/** The rail surface: the list, or the Session open on it. */
export function SessionsSurface({ openId, onOpen, goes }: { openId: string | null; onOpen: (id: string) => void; goes: LedgerGoes }) {
  const sessions = useSessions();
  const open = sessions.find((one) => one.id === openId);
  return (
    <div className="armada-screen__overview">
      {open === undefined ? <SessionsListing onOpen={onOpen} /> : <SessionView session={open} goes={goes} onOpen={onOpen} />}
    </div>
  );
}
