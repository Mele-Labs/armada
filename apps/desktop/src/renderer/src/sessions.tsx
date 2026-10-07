// Sessions, as the window draws them: a rail surface, a panel on Overview, and
// who owns a chip anywhere. Apart from `App.tsx`, which is at its length.
// What is drawn is `@armada/components`'; this reads the draft
// (`packages/screens/src/draft/sessions.ts`) into it.

import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  ChipOwnership,
  SessionFrame,
  SessionLedger,
  SessionList,
  SessionThread,
  hueOf,
} from "@armada/components";
import type {
  ChipOwnershipValue,
  LedgerEntry,
  OwnerChipRef,
  OwnerSummary,
  SessionRowView,
  SessionState,
  SessionThreadRow,
} from "@armada/components";
import { attachmentsOf, isBlank, ownerOf, sessionsMatching } from "@armada/screens/src/draft/sessions";
import type { ChipRef, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import { SURFACE } from "@armada/shell";

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

const checksSaid = (checks: { state: "pending" | "passed" | "failed"; failing?: string }) =>
  checks.state === "failed" ? `Checks failed: ${checks.failing}` : checks.state === "pending" ? "Checks running" : "Checks passed";

function summaryOf(session: Session): OwnerSummary {
  return {
    id: session.id,
    ...(session.title === undefined ? {} : { title: session.title }),
    hue: hueOf(stateOf(session).state),
    slots: attachmentsOf(session, "slot").map((one) => one.slot),
    pullRequests: attachmentsOf(session, "pull_request").map((one) => ({
      number: one.number,
      checks: one.checks.state,
      said: checksSaid(one.checks.state === "failed" ? one.checks : { state: one.checks.state }),
    })),
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

/** Every Session, searchable, with the act that starts one. On Overview and on the rail surface alike. */
export function SessionsListing({ onOpen }: { onOpen: (id: string) => void }) {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const [query, setQuery] = useState("");
  if (draft === undefined) return null;
  const rows: SessionRowView[] = sessionsMatching(sessions, query).map(({ session, matched }) => {
    const { state, said } = stateOf(session);
    const chip = matched === "title" || matched === "id" ? undefined : chipOf(matched);
    return {
      id: session.id,
      ...(session.title === undefined ? {} : { title: session.title }),
      state,
      said,
      ...(chip === undefined ? {} : { matched: chip }),
      ...(session.lastTurn === undefined ? {} : { lastTurn: session.lastTurn }),
    };
  });
  return <SessionList rows={rows} query={query} onQuery={setQuery} onOpen={onOpen} onStart={() => onOpen(draft.start())} />;
}

function threadRowsOf(session: Session): SessionThreadRow[] {
  return session.rows.map((row): SessionThreadRow => {
    if (row.kind === "lease" || row.kind === "tool") return row;
    return row.from.kind === "session"
      ? { id: row.id, at: row.at, kind: "message", from: "session", sender: { id: row.from.id, title: row.from.title }, text: row.text }
      : { id: row.id, at: row.at, kind: "message", from: row.from.kind, text: row.text };
  });
}

/** Where a ledger row goes: the surface that already shows the thing. */
export type LedgerGoes = {
  onOpenJob: (jobId: string) => void;
  onGoTo: (surfaceId: string) => void;
  onOpenLink: (address: string) => void;
};

function entriesOf(session: Session, goes: LedgerGoes): LedgerEntry[] {
  return session.attachments.map((one): LedgerEntry => {
    switch (one.kind) {
      case "slot":
        return { key: `slot${one.slot}`, kind: "slot", name: `Slot ${one.slot}`, text: <code>{one.slot}</code>, onOpen: () => goes.onGoTo(SURFACE.worktrees) };
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
          mark: { glyph: one.checks.state, said: checksSaid(one.checks.state === "failed" ? one.checks : { state: one.checks.state }) },
          onOpen: () => goes.onOpenLink(one.address),
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
      case "subagent":
        return {
          key: one.id,
          kind: "subagent",
          name: `Subagent ${one.task}, ${one.state}`,
          text: one.task,
          mark: { glyph: one.state, said: one.state === "running" ? "Running" : "Done" },
          onOpen: () => undefined,
        };
    }
  });
}

/** One Session open: its conversation in the middle and what it holds at the side, one view. */
function SessionView({ session, goes }: { session: Session; goes: LedgerGoes }) {
  const draft = useSessionsDraft();
  const { state, said } = stateOf(session);
  return (
    <SessionFrame state={state} said={said} id={session.id} {...(session.title === undefined ? {} : { title: session.title })}>
      <SessionThread
        rows={threadRowsOf(session)}
        {...(session.asked === undefined ? {} : { asked: session.asked })}
        onAnswer={() => draft?.answer(session.id)}
        working={session.turn.state === "working"}
        onSend={(text) => draft?.send(session.id, text)}
      />
      <SessionLedger entries={entriesOf(session, goes)} />
    </SessionFrame>
  );
}

/** The rail surface: the list, or the Session open on it. */
export function SessionsSurface({ openId, onOpen, goes }: { openId: string | null; onOpen: (id: string) => void; goes: LedgerGoes }) {
  const sessions = useSessions();
  const open = sessions.find((one) => one.id === openId);
  return (
    <div className="armada-screen__overview">
      {open === undefined ? <SessionsListing onOpen={onOpen} /> : <SessionView session={open} goes={goes} />}
    </div>
  );
}

