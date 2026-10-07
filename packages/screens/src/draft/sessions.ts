// A Session: a raw, open-ended conversation with a hosted agent, with Armada's
// features hooked in. Draft, mock-only: nothing in Fleet or the protocol
// carries it yet, so every field here is one Fleet would owe. The owner's ask,
// 6 Oct 2026.
//
// **A Session sits beside Helm and changes nothing about it.** Helm stays the
// one dock per repository. A Session is a thread of its own that starts with
// no slot and no branch, leases a worktree slot on the agent's first write,
// and from then on accumulates what it touches.

/** What a Session has attached, by kind. A Session holds several of each. */
export type SessionAttachment =
  /** A worktree slot the Session holds: leased on its first write, never at birth. */
  | { kind: "slot"; slot: number }
  | { kind: "branch"; name: string; slot: number }
  | { kind: "pull_request"; number: number; title: string; branch: string; address: string; checks: SessionChecks }
  /** A Job the Session dispatched. It leases its own slot and cuts its own branch. */
  | { kind: "job"; id: string; number: number; title: string; state: SessionJobState; branch: string; slot: number }
  | { kind: "studio"; id: string; title: string }
  | { kind: "subagent"; id: string; task: string; state: "running" | "done" };

export type SessionChecks = { state: "pending" } | { state: "passed" } | { state: "failed"; failing: string };

export type SessionJobState = "running" | "review" | "landed";

export type SessionAttachmentKind = SessionAttachment["kind"];

/** Who said a row of a Session's thread. **A Session that wrote to this one is named**, never folded into "agent". */
export type SessionVoice =
  | { kind: "you" }
  | { kind: "agent" }
  | { kind: "session"; id: string; title: string };

export type SessionRow =
  | { id: string; at: string; kind: "message"; from: SessionVoice; text: string }
  /** A tool call, mono. */
  | { id: string; at: string; kind: "tool"; text: string }
  /** The agent's first write: the slot leased and the branch cut, drawn in the thread where it happened. */
  | { id: string; at: string; kind: "lease"; slot: number; branch: string };

/** A turn running, or none. A message from another Session starts one, so `working` has no author. */
export type SessionTurn = { state: "idle" } | { state: "working"; wokenBy?: { id: string; title: string } };

export type Session = {
  id: string;
  /** Absent until the first turn has named it; a blank Session is known by its id alone. */
  title?: string;
  attachments: readonly SessionAttachment[];
  rows: readonly SessionRow[];
  turn: SessionTurn;
  /** When the last turn ended, already worded. Absent on a Session that has not had one. */
  lastTurn?: string;
  /** What the agent is held on, while it is. */
  asked?: SessionAsk;
};

/** A permission a Session's agent is held on. */
export type SessionAsk = { command: string };

/**
 * What the window holds of Sessions, and the acts on them. **The mock's seam**
 * (`apps/desktop/src/renderer/src/sessions-draft.tsx`): a real Fleet gives none,
 * so a surface that reads this draws nothing and the rail row is left off. When
 * Fleet serves Sessions this becomes a read and these acts become commands.
 */
export type SessionsDraft = {
  get: () => readonly Session[];
  subscribe: (onChange: () => void) => () => void;
  /** Starts a blank Session and returns its id. It holds no slot and no branch until the agent writes. */
  start: () => string;
  /** A message from the person. It takes a turn. */
  send: (id: string, text: string) => void;
  /** Answers the permission a Session is held on. */
  answer: (id: string) => void;
};

/** Anything a chip anywhere in Bridge can name. A chip asks who owns it. */
export type ChipRef =
  | { kind: "branch"; name: string }
  | { kind: "pull_request"; number: number }
  | { kind: "slot"; slot: number }
  | { kind: "job"; id: string };

/** Whether a Session has leased anything yet. A blank one has no slot and no branch. */
export function isBlank(session: Session): boolean {
  return session.attachments.length === 0 && session.rows.length === 0;
}

export function attachmentsOf<K extends SessionAttachmentKind>(
  session: Session,
  kind: K,
): Extract<SessionAttachment, { kind: K }>[] {
  return session.attachments.filter((one): one is Extract<SessionAttachment, { kind: K }> => one.kind === kind);
}

function names(attachment: SessionAttachment, ref: ChipRef): boolean {
  switch (ref.kind) {
    case "branch":
      return (
        (attachment.kind === "branch" && attachment.name === ref.name) ||
        (attachment.kind === "job" && attachment.branch === ref.name) ||
        (attachment.kind === "pull_request" && attachment.branch === ref.name)
      );
    case "pull_request":
      return attachment.kind === "pull_request" && attachment.number === ref.number;
    case "slot":
      return (
        (attachment.kind === "slot" && attachment.slot === ref.slot) ||
        (attachment.kind === "job" && attachment.slot === ref.slot)
      );
    case "job":
      return attachment.kind === "job" && attachment.id === ref.id;
  }
}

/** The Session that owns what a chip names, or none. **At most one**: a slot, a branch and a PR each belong to one. */
export function ownerOf(sessions: readonly Session[], ref: ChipRef): Session | undefined {
  return sessions.find((session) => session.attachments.some((one) => names(one, ref)));
}

/** A Session found by a search, and what in it matched, so a list can say why it is there. */
export type SessionHit = { session: Session; matched: SessionAttachment | "title" | "id" };

const NUMBER = /^\d+$/;

/**
 * The Sessions a query finds, by PR number, branch, Job id, slot or title.
 *
 * **`#1843` is a pull request and nothing else**, as a leading `#` is everywhere
 * in Armada. `slot 3` and `job 52` say their kind; a bare number is any of the
 * three, a PR by prefix of its number.
 */
export function sessionsMatching(sessions: readonly Session[], query: string): SessionHit[] {
  const said = query.trim().toLowerCase();
  if (said === "") return sessions.map((session) => ({ session, matched: "id" }));
  const hash = said.startsWith("#");
  const kinded = /^(slot|job|pr)\s*#?\s*(\S+)$/.exec(said);
  const word = hash ? said.slice(1) : kinded !== null ? kinded[2]! : said;
  const only: SessionAttachmentKind | undefined = hash
    ? "pull_request"
    : kinded === null
      ? undefined
      : ({ slot: "slot", job: "job", pr: "pull_request" } as const)[kinded[1] as "slot" | "job" | "pr"];

  const hits: SessionHit[] = [];
  for (const session of sessions) {
    const attached = session.attachments.find((one) => {
      if (only !== undefined && one.kind !== only && !(only === "slot" && one.kind === "job")) return false;
      switch (one.kind) {
        case "pull_request":
          return String(one.number).startsWith(word.replace(/^#/, ""));
        case "branch":
          return only === undefined && one.name.toLowerCase().includes(word);
        case "slot":
          return NUMBER.test(word) && one.slot === Number(word);
        case "job":
          return only === "slot"
            ? NUMBER.test(word) && one.slot === Number(word)
            : (only === undefined || only === "job") && (one.id === word || one.branch.toLowerCase().includes(word));
        default:
          return false;
      }
    });
    if (attached !== undefined) hits.push({ session, matched: attached });
    else if (only === undefined && session.title?.toLowerCase().includes(word)) hits.push({ session, matched: "title" });
  }
  return hits;
}
