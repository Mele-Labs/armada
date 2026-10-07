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
  /** `handed` is a slot that came with a Job a person piloted, which the Session did not lease. */
  | { kind: "slot"; slot: number; handed?: { job: number } }
  | { kind: "branch"; name: string; slot: number; handed?: { job: number } }
  | {
      kind: "pull_request";
      number: number;
      title: string;
      branch: string;
      address: string;
      checks: SessionChecks;
      /** A draft is not ready for review; a merged one has landed. */
      state: SessionPullRequestState;
      /** Merge once every Check passes, asked for while they were still running. */
      auto: boolean;
    }
  /** A Job the Session dispatched. It leases its own slot and cuts its own branch. */
  | {
      kind: "job";
      id: string;
      number: number;
      title: string;
      state: SessionJobState;
      branch: string;
      /** Absent where the Job's own slot is not known: Fleet holds it on the Job, not on the Session. */
      slot?: number;
      /** A Job the person tagged, which the Session is looking at. Absent is one it dispatched. */
      looking?: true;
      /** Closed by a person's word and not by its gates. */
      attested?: true;
      /** The Job is piloted, but from another Session: this one's ledger offers no exit on it. Absent is this Session's own. */
      pilotedElsewhere?: true;
    }
  | { kind: "studio"; id: string; title: string }
  /** A sketch the agent published into the Session, or one the person shared with it. */
  | { kind: "sketch"; id: string; title: string; by: "agent" | "you"; drawing: SessionSketch }
  | { kind: "subagent"; id: string; task: string; state: "running" | "done"; report?: string }
  /**
   * Something the Session made that a person would open: a page it published, a file it wrote outside
   * the code, or a Doc. `id` is the address of a page or document, or the path of a file.
   * **A code edit is never one**; those are Branches and Pull requests.
   */
  | { kind: "artifact"; form: SessionArtifactForm; id: string; title: string }
  /** The Session this one was forked to, and the one it was forked from. `id` is the other Session's. */
  | { kind: "forked_to"; id: string }
  | { kind: "forked_from"; id: string };

export type SessionArtifactForm = "page" | "file" | "doc";

/**
 * A sketch as Dispatch and Studios hold one: boxes and the joins between them.
 * **The same shape the pad draws** (`SketchBox`, `SketchLine`), so the ledger
 * reads it with the preview Studios already draws.
 */
export type SessionSketch = {
  boxes: readonly { id: string; x: number; y: number; body: string }[];
  lines: readonly { id: string; from: string; to: string }[];
  /** Lines drawn by hand. Absent is none. */
  strokes?: readonly { id: string; points: readonly { x: number; y: number }[] }[];
};

/** A sketch drawn for a message, the one the pad was left holding. */
export type DrawnSketch = { id: string; title: string; drawing: SessionSketch };

/** The permission modes a Session runs in, as the terminal's are: ask, auto, accept edits, plan. */
export type SessionMode = "ask" | "auto" | "accept_edits" | "plan";

/** A file or picture sent with a message. `src` is a `blob:` address for a picture. */
export type SentFile = {
  id: string;
  name: string;
  src?: string;
  /** What was picked. A host that sends the file reads its bytes from here. */
  file?: File;
};

export type SessionPullRequestState = "draft" | "open" | "merged";

/** What a person can do to a pull request without leaving Bridge. */
export type PullRequestAct = "ready" | "merge" | "auto_merge" | "review";

export type SessionChecks = { state: "pending" } | { state: "passed" } | { state: "failed"; failing: string };

/** `piloted` is a Job the person is working in this Session, its Drone stopped. */
export type SessionJobState = "running" | "review" | "landed" | "escalated" | "piloted" | "superseded";

export type SessionAttachmentKind = SessionAttachment["kind"];

/** Who said a row of a Session's thread. **A Session that wrote to this one is named**, never folded into "agent". */
export type SessionVoice =
  | { kind: "you" }
  | { kind: "agent" }
  | { kind: "session"; id: string; title: string };

export type SessionRow =
  | {
      id: string;
      at: string;
      kind: "message";
      from: SessionVoice;
      text: string;
      /** What the person sent with it. */
      files?: readonly SentFile[];
      sketches?: readonly { id: string; title: string }[];
      /** Sessions tagged with `@`, so the agent knows to talk to them. */
      tags?: readonly SessionTag[];
    }
  /** A tool call, mono. */
  | { id: string; at: string; kind: "tool"; text: string }
  /** A command the person ran in the terminal, as typed. */
  | { id: string; at: string; kind: "command"; text: string }
  /** The summary the CLI wrote where it compacted the conversation: not the person's words. */
  | { id: string; at: string; kind: "compaction"; text: string }
  /**
   * What a piloted Session starts with: the Job's own worktree, handed over,
   * and what Fleet knew when its Drone stopped. Structured, never prose.
   */
  | {
      id: string;
      at: string;
      kind: "handoff";
      job: { number: number; title: string };
      slot: number;
      branch: string;
      step?: { id: string; label: string };
      attempts: number;
      refusals: readonly string[];
      plan: { outside: readonly string[]; unwritten: readonly string[] };
      narrative?: { trying_to: string; blocked_by: string; tried: readonly string[] };
    }
  /** The agent's first write: the slot leased and the branch cut, drawn in the thread where it happened. */
  | { id: string; at: string; kind: "lease"; slot: number; branch: string };

/** A turn running, or none. A message from another Session starts one, so `working` has no author. */
export type SessionTurn = { state: "idle" } | { state: "working"; wokenBy?: { id: string; title: string } };

export type Session = {
  id: string;
  /** What other sessions call it, `s-` and the first eight characters of the id. Drawn where the id would be. */
  address?: string;
  /**
   * Nothing more can be said to it: it `ended`, or it runs in a terminal whose mod has stopped asking
   * (`quiet`). Such a Session offers Fork in place of a message box, and owns nothing.
   */
  dead?: "ended" | "quiet";
  /** Ended more than a week ago: left off the list, found by search. */
  older?: true;
  /** A session from a terminal: its thread is read from the terminal's transcript, and it is sent words and nothing else. */
  terminal?: true;
  /** The commands a terminal session lists, where its mod has said. A hosted one uses the draft's. */
  commands?: readonly SessionCommand[];
  /** Set where the rows are not all held, so a session whose thread was never opened is not taken for a blank one. */
  blank?: boolean;
  /** Absent until the first turn has named it; a blank Session is known by its id alone. */
  title?: string;
  attachments: readonly SessionAttachment[];
  rows: readonly SessionRow[];
  turn: SessionTurn;
  /** When the last turn ended, already worded. Absent on a Session that has not had one. */
  lastTurn?: string;
  /** The same instant, for the list's "4m" and "2h". Absent where only the clock is known. */
  lastTurnAt?: string;
  /** What the agent is held on, while it is. */
  asked?: SessionAsk;
  /** Tags chosen and not yet sent: they wait in the message box as chips. */
  pendingTags?: readonly SessionTag[];
  /** The model and effort the next turn runs on. Absent is Auto. */
  model?: string;
  effort?: string;
  /** The permission mode. **A Session runs in auto**, so absent is auto. */
  mode?: SessionMode;
};

/**
 * Something a message tags with `@`, so the agent knows what is meant: another
 * Session, a Job, a pull request or a branch. A Job carries what the ledger
 * needs to hold it as one the Session is looking at.
 */
export type SessionTag = {
  kind: "session" | "job" | "pull_request" | "branch";
  id: string;
  title: string;
  /** `slot` is absent where the Job's own slot is not known to the window: Fleet holds it on the Job. */
  job?: { number: number; branch: string; slot?: number; state: SessionJobState };
};

/** What a message carries: its words, and what was attached to it. */
export type SentMessage = {
  text: string;
  files: readonly SentFile[];
  /** Sketches drawn for this message. */
  sketches: readonly DrawnSketch[];
  tags: readonly SessionTag[];
};

/** A skill or command `/` offers, as a terminal session lists them. */
export type SessionCommand = { name: string; says: string };

/** What a person may answer a permission a Session is held on. */
export type SessionAnswer = "allow_once" | "allow_and_remember" | "refuse";

/**
 * A permission a Session's agent is held on. `call` is Fleet's id for it and `offers` the answers it
 * will take, in the order to draw them. Absent on the mock's, which offers two.
 */
export type SessionAsk = { command: string; call?: string; offers?: readonly SessionAnswer[] };

/**
 * What the window holds of Sessions, and the acts on them. **The mock's seam**
 * (`apps/desktop/src/renderer/src/sessions-draft.tsx`): a real Fleet gives none,
 * so a surface that reads this draws nothing and the rail row is left off. When
 * Fleet serves Sessions this becomes a read and these acts become commands.
 */
export type SessionsDraft = {
  get: () => readonly Session[];
  subscribe: (onChange: () => void) => () => void;
  /**
   * Takes a Job's worktree: starts a Session on it and returns the Session's id. **A promise on a real
   * Fleet**, one call that takes the Job over and starts the Session; one Fleet refused says why in
   * `said`. Absent where nothing serves it, and the acts are left off rather than drawn dead.
   */
  pilot?: (jobId: string, outcome: "take_over" | "restart_step") => string | Promise<string | undefined>;
  /**
   * Starts a new Session as a copy of a dead one's conversation and returns its id, as `pilot` does.
   * Absent where nothing serves it, and Fork is left off rather than drawn dead.
   */
  fork?: (id: string) => string | Promise<string | undefined>;
  /** One of the three ways out of a pilot. Absent with `pilot`. A refusal is said in `said`. */
  exit?: (jobId: string, exit: "submit" | "attest" | "supersede") => void;
  /**
   * Starts a blank Session and returns its id. It holds no slot and no branch until the agent writes.
   * **A promise on a real Fleet**, which names the Session; one that could not start says why in `said`.
   */
  start: (tag?: SessionTag) => string | Promise<string | undefined>;
  /** Opens a Session's thread for reading. Absent where the thread is already held. */
  watch?: (id: string) => void;
  /** Ends a Session: the slot is parked and the row ends. Absent in the mock, which has no end. */
  close?: (id: string) => void;
  /** Names a Session, hosted or in a terminal. Absent where there is nothing to save it to. */
  rename?: (id: string, title: string) => void;
  /** Opens a file a Session wrote. Absent in the mock, whose files are not on this machine. */
  openFile?: (id: string, path: string) => void;
  /** Reads a pull request again, so its Checks are what the forge says now. Absent in the mock. */
  refresh?: (id: string, number: number) => void;
  /** What Fleet refused, in words, until the next act. Absent in the mock. */
  said?: () => string | undefined;
  /** What `@` offers beyond other Sessions: Jobs, pull requests and branches. */
  taggable: () => readonly SessionTag[];
  /** Puts a Session's waiting tags where the person left them. */
  setTags: (id: string, tags: readonly SessionTag[]) => void;
  /** A message from the person, with what they sent along. It takes a turn. */
  send: (id: string, sent: SentMessage) => void;
  /** Sets the model, effort and permission mode a Session's next turn runs on. `null` is Auto. */
  tune: (id: string, tuning: { model: string | null; effort: string | null; mode: SessionMode }) => void;
  /** An act on one of a Session's pull requests. `review` dispatches a Job on the code review workflow against it. */
  act: (id: string, number: number, act: PullRequestAct) => void;
  /** Set where `act` dispatches the review itself, so the window does not propose one beside it. */
  reviews?: "fleet";
  /** The models and efforts a Session may be set to, as Dispatch offers them. */
  models: readonly string[];
  efforts: readonly string[];
  /** The skills and commands `/` offers. */
  commands: readonly SessionCommand[];
  /** Answers the permission a Session is held on. The mock offers two and names none. */
  answer: (id: string, answer?: SessionAnswer) => void;
};

/** Anything a chip anywhere in Bridge can name. A chip asks who owns it. */
export type ChipRef =
  | { kind: "branch"; name: string }
  | { kind: "pull_request"; number: number }
  | { kind: "slot"; slot: number }
  | { kind: "job"; id: string };

/** Whether a Session has leased anything yet. A blank one has no slot and no branch. */
export function isBlank(session: Session): boolean {
  return session.blank ?? (session.attachments.length === 0 && session.rows.length === 0);
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
        (attachment.kind === "job" && attachment.looking !== true && attachment.branch === ref.name) ||
        (attachment.kind === "pull_request" && attachment.branch === ref.name)
      );
    case "pull_request":
      return attachment.kind === "pull_request" && attachment.number === ref.number;
    case "slot":
      return (
        (attachment.kind === "slot" && attachment.slot === ref.slot) ||
        (attachment.kind === "job" && attachment.looking !== true && attachment.slot === ref.slot)
      );
    case "job":
      return attachment.kind === "job" && attachment.looking !== true && attachment.id === ref.id;
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
