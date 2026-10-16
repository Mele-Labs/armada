// A Session: a raw, open-ended conversation with a hosted agent, with Armada's
// features hooked in. Draft, mock-only: nothing in Fleet or the protocol
// carries it yet, so every field here is one Fleet would owe. The owner's ask,
// 6 Oct 2026.
//
// **A Session sits beside Helm and changes nothing about it.** Helm stays the
// one dock per repository. A Session is a thread of its own that starts with
// no slot and no branch, leases a worktree slot on the agent's first write,
// and from then on accumulates what it touches.

import type { ContextBreakdown } from "@armada/components/src/context-breakdown";
import type { Scene } from "@armada/components/src/compositions/SketchScene/scene";
import type { PullRequestFilePatch, PullRequestPage, SessionBuild, SessionStanding, ThoughtTarget, WaitingItem, WritePullRequest } from "@armada/protocol";
import type { RetroRowDrawn } from "../retro-row";

/** When a Session's work happened, each as an ISO instant. `until` is a slot given back. */
export type SessionTrail = {
  /** `SessionRecord.started_at`. */
  started: string;
  /** The slots it held, oldest first: a `slot` attachment's `since` and, once given back, `changed_at`. */
  bays?: readonly { slot: number; from: string; until?: string }[];
  /** When each pull request was recorded, by number: a `pr` attachment's `since`. */
  opened?: Readonly<Record<number, string>>;
};

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
      /** What its author wrote of it, as Markdown. Absent where there is none. */
      body?: string;
      checks: SessionChecks;
      /** A draft is not ready for review; a merged one has landed. */
      state: SessionPullRequestState;
      /** Merge once every Check passes, asked for while they were still running. */
      auto: boolean;
      /** In the base branch's merge queue now, which is more than asking for it. */
      queued?: true;
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
  /**
   * A sketch the agent published into the Session, or one the person shared with it.
   * **The agent's is a scene, and a second one with the same title replaces the first under the same `id`.**
   * Its thread card is derived from the `show_sketch` call's tool row (`sketchCallOf`), never a row of its own.
   * `against` is the sketch it is read as a change to, held whole so a later replacement does not move it.
   */
  | { kind: "sketch"; id: string; title: string; by: "agent" | "you"; drawing: SessionSketch | Scene; against?: { title: string; drawing: SessionSketch | Scene } }
  | { kind: "subagent"; id: string; task: string; state: "running" | "done"; report?: string }
  /**
   * Something the Session made that a person would open: a page it published, a file it wrote outside
   * the code, or a Doc. `id` is the address of a page or document, or the path of a file.
   * **A code edit is never one**; those are Branches and Pull requests.
   */
  | { kind: "artifact"; form: SessionArtifactForm; id: string; title: string }
  /**
   * The Session this one was forked to, and the one it was forked from. `id` is the other Session's.
   * `moved` is present where the fork was a move to another repository: the id of the repository on the other side.
   */
  | { kind: "forked_to"; id: string; moved?: string }
  /**
   * `at` is the message the fork was taken from: the old conversation is shown up to and including it. Absent, the whole of it.
   * `moved` is the repository the Session was moved from and `branch` the branch it left there, comma-separated.
   */
  | { kind: "forked_from"; id: string; at?: string; moved?: string; branch?: string }
  /** A Session this one's agent started when asked, and the Session whose agent started this one. `id` is the other Session's. */
  | { kind: "started_to"; id: string }
  | { kind: "started_by"; id: string };

/** A `window` is a page the Session showed in a window of Bridge's own; `id` is its address. */
export type SessionArtifactForm = "page" | "file" | "image" | "doc" | "window";

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

// ---- What a Session works to ---------------------------------------------------------------------------
// The plan, the workflow, the subagents on the field and the judges' concerns, drawn in the Session's own view on
// the Cockpit and on the Sessions page alike, from the record's plan, workflow, proposal and touches (`session-work.ts`). Progress is the agent's
// task states with what Fleet observed beside them (`seen`), and a status ring no longer stands for it.

export type WorkTaskState = "open" | "working" | "handed_in" | "done" | "failed";

/** One task the agent recorded. `seen` is whether Fleet observed edits for it. */
export type WorkTask = { id: string; text: string; state: WorkTaskState; seen?: boolean };

/** Tasks one subagent takes, by the type of subagent. */
export type WorkGroup = { id: string; agent: string; title: string; /** The paths this group works in. */ paths: readonly string[]; tasks: readonly WorkTask[] };

/** The scope as paths, the groups, and whether the person has approved it. */
/** A sketch the agent drew with the plan, in the format `show_sketch` takes. `id` is the sketch on the Session's ledger, which opens it. */
export type WorkSketch = { id: string; title: string; scene: Scene };

/**
 * What he marked on the plan before it goes to the agent: renamed, removed, added, moved, a group's subagent type changed, or a
 * note pinned to a group or a task. Nothing is changed until he sends them; they ride on the plan as marks. `group` is a group's id and
 * `task` a task's id; an `add` with no `task` is a group.
 */
export type PlanMark =
  | { kind: "rename"; group: string; task?: string; from: string; to: string }
  | { kind: "remove"; group: string; task?: string }
  | { kind: "add"; group: string; task?: string; id: string; text: string }
  | { kind: "move"; task: string; from: string; to: string }
  | { kind: "agent"; group: string; from: string; to: string }
  | { kind: "note"; id: string; group: string; task?: string; text: string };

/** One line of what he sent, against the group or task it is about. */
export type SuggestLine = { verb: "Renamed" | "Removed" | "Added" | "Moved" | "Subagent" | "Note"; where: string; text?: string; from?: string };

/** What the agent changed in the plan it recorded again: ids of what is new, what was renamed and what it was, and what is gone. */
export type PlanChanges = { added: readonly string[]; renamed: readonly { id: string; from: string }[]; removed: readonly { group: string; text: string }[] };

export type WorkPlan = { /** The version the agent recorded, which approving and sending changes name. */ version?: number; scope: readonly string[]; groups: readonly WorkGroup[]; approved: boolean; /** What changed since the plan before this one. */ changes?: PlanChanges; /** The sketch drawn with the plan, if the agent drew one. */ sketch?: WorkSketch; /** The thread row that recorded it, which the approved plan settles after. */ after?: string };

/** A drone of this Session over the area it is working in: the Session's own, or a subagent of `agent` type. */
export type WorkDrone = { id: string; agent?: string; task?: string; path: string; doing: "read" | "write" | "run"; /** The file it is over, which the Field draws as a bar. */ file?: string };

/** A file of the repository as the Field draws it, a bar on the plot of its directory. `act` is what is being done to it now: made, changed or taken away. */
export type WorkFile = { path: string; act?: "write" | "edit" | "remove" };

/** A concern a judge raised at a step. It waits on the person; a judge never blocks. */
export type WorkConcern = { id: string; /** The step it was raised after, where a boundary judge raised it; the Judge tile's carry none. */ step?: string; judge?: string; text: string; /** What it cites: a file, and `:line` where it names one. */ cites?: readonly string[] };

/** A workflow a Session may follow: its steps in order. */
export type WorkflowChoice = { id: string; name: string; steps: readonly string[]; /** The repository that holds it. Absent is any. */ manifestId?: string };

export type SessionWork = {
  /** What the proposer read the request to be. Changeable at any time; a change applies from the next turn. */
  proposal?: { /** The id of the workflow chosen. */ workflow: string | null; /** The proposer's call is out: the Session starts on its answer. */ reading?: boolean; /** What the proposer picked and the person has not changed: each is marked where it lives. */ picked?: readonly ("workflow" | "model" | "effort")[] };
  /** The workflow's steps and the index of the one the Session is at, where Fleet says. Absent is a custom Session, which has no rail; `at` absent draws the steps unlit. */
  workflow?: { name: string; steps: readonly string[]; at?: number };
  /** Absent until the agent records its plan after planning with the person. */
  plan?: WorkPlan;
  field?: readonly WorkDrone[];
  /** The files the Field draws as bars on its plots. */
  files?: readonly WorkFile[];
  concerns?: readonly WorkConcern[];
  /** A judge is reading the work now. */
  judging?: true;
  /** The last reading's outcome: `concerns` 0 is how the tile says "No concerns". */
  judged?: { at: string; concerns: number };
  /** The owner's setting turned workflows off: the picker, the steps and the proposer's mark for a workflow are not drawn. */
  railOff?: true;
};

/** The acts on a Session's work. Absent where nothing serves them. */
export type SessionWorkActs = {
  workflows: readonly WorkflowChoice[];
  /** Changes the workflow at any time, from the next turn; `null` is none, a custom Session. */
  choose: (id: string, workflow: string | null) => void;
  /** The person changed a value the proposer picked, so its mark goes. */
  unpick: (id: string, value: "workflow" | "model" | "effort") => void;
  approve: (id: string) => void;
  /** Sends the marks he put on the plan to the agent as one message, which answers and may record the plan again. `approving` is Approve pressed with marks on: the plan is approved with them. */
  suggest: (id: string, marks: readonly PlanMark[], approving?: boolean) => void;
  /** Sends the concern to the Session as a message to the agent. */
  sendConcern: (id: string, concern: string) => void;
  dismissConcern: (id: string, concern: string) => void;
  /** Sends a judge to read the Session's work so far against its plan. Fleet answers at once; the reading lands as `judging`, then `judged` and `concerns`. */
  judge?: (id: string) => Promise<void>;
};

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

/**
 * `failed` is the required check red. `red` on the other two names a check that is red and not required:
 * on `pending` while the run goes on, on `passed` after the required checks passed.
 */
export type SessionChecks = { state: "pending"; red?: string } | { state: "passed"; red?: string } | { state: "failed"; failing: string };

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
      /** The instant `at` is the clock of, where the wire gave one. A mock's carries none. */
      instant?: string;
      from: SessionVoice;
      text: string;
      /** What the person sent with it. */
      files?: readonly SentFile[];
      sketches?: readonly { id: string; title: string }[];
      /** Sessions tagged with `@`, so the agent knows to talk to them. */
      tags?: readonly SessionTag[];
    }
  /**
   * Words typed in Helm's box that went to a Session or a Job and never to Helm. Only Helm's own Session
   * has these: Fleet keeps the row beside the ones the agent wrote, in the thread where it was sent.
   */
  | {
      id: string;
      at: string;
      kind: "sent";
      text: string;
      /** The Session or Job the words went to, and the repository it is of as the picker names it. */
      target: ThoughtTarget & { repository: string };
      files?: readonly SentFile[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly SessionTag[];
    }
  /** A tool call, mono. `fromPhone` is an ask the person answered on their phone. */
  | { id: string; at: string; kind: "tool"; text: string; fromPhone?: true }
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
  | { id: string; at: string; kind: "lease"; slot: number; branch: string }
  /** A value the person changed through a panel, which the agent was told of: what it was and what it is now. */
  | { id: string; at: string; kind: "change"; what: string; from: string; to: string }
  /** A judge's concern he dismissed: one settled line, and the agent was told nothing. */
  | { id: string; at: string; kind: "concern"; step: string; text: string }
  /** The changes and notes he sent on the plan, one message with a line for each. */
  | { id: string; at: string; kind: "suggest"; lines: readonly SuggestLine[] }
  /** An `armada check` the agent ran in its slot: the Check's key, its run on the Checks page, and where it stands. */
  | { id: string; at: string; kind: "check"; name: string; run: number; state: "running" | "passed" | "failed"; target?: string }
  /** A command the person ran from the thread or the band, which the Session was told: its exit (`code` is null where it was stopped) and the last lines it printed, as plain text. */
  | { id: string; at: string; kind: "ran"; command: string; code: number | null; tail: string }
  /** The Session showed a page in a window: where in the thread it did, and the address to open again. */
  | { id: string; at: string; kind: "window"; title: string; url: string }
  /** This Session's agent started a Session, or forked itself into one: the new Session, to open from the thread. */
  | { id: string; at: string; kind: "started"; session: { id: string; title: string }; fork: boolean }
  /** A retro being written for the Session, then the one that was: the latest frame of one press's row. */
  | RetroRowDrawn;

/** One item of the agent's own TodoWrite list. The agent's report of itself; `SessionActivity.now` is what Fleet detected. */
export type SessionTodo = { text: string; state: "done" | "doing" | "todo" };

/**
 * What a Session is doing, from Fleet's PreToolUse gate: `now` is the last tool call as a short
 * action, and `todos` the agent's list where it keeps one. Todos are detail on top of `now`, never a
 * plan the Session is held to.
 */
export type SessionActivity = { now?: string; todos?: readonly SessionTodo[] };

/** A turn running, or none. A message from another Session starts one, so `working` has no author. */
export type SessionTurn = { state: "idle" } | { state: "working"; wokenBy?: { id: string; title: string } };

/** One ledger row a release moved or left: its kind as the ledger names it (`slot`, `branch`, `pr`, `job`, `need`) and its target. */
export type HeldRow = {
  kind: string;
  target: string;
  /** A slot given back: the branch its work is on, and the WIP commit and files where something was uncommitted. */
  saved?: { branch: string; commit?: string; files: readonly string[] };
};

/** What a release gave back or handed over, and what stayed with why. `to` is the Session handed them. */
export type HoldsReleased = { to?: string; moved: readonly HeldRow[]; kept: readonly (HeldRow & { why: string })[] };

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
  /** Its mod is older than the repository's, or reported no version. */
  modOutOfDate?: true;
  /** The commands a terminal session lists, where its mod has said. A hosted one uses the draft's. */
  commands?: readonly SessionCommand[];
  /** Set where the rows are not all held, so a session whose thread was never opened is not taken for a blank one. */
  blank?: boolean;
  /** Held until the plan's limit resets, as ISO: Fleet wakes it then. Absent unless it is held. */
  limitedUntil?: string;
  /** How full its context window is, as last reported. Absent until it has reported one. */
  context?: { tokens: number; window: number };
  /** What fills that window, as the harness reports it. Absent where it has not. */
  contextBreakdown?: ContextBreakdown;
  /**
   * Helm's own Session for a repository: hosted like any other, pinned to Helm's dock, one per
   * repository. The Sessions list marks it and opening it opens the dock, not a page of its own.
   * It runs in auto, so `mode` is never offered for it.
   */
  helm?: true;
  /** Absent until the first turn has named it; a blank Session is known by its id alone. */
  title?: string;
  /** The repository it works in, by Manifest id: what `#` reads issues and pull requests of. */
  manifestId?: string;
  /** The directory it was started in, `SessionRecord.cwd`. A report about it names where its transcript is kept. */
  cwd?: string;
  attachments: readonly SessionAttachment[];
  /** The needs it holds standing, by repository-relative path and the words it declared each in. Absent is none. */
  needs?: readonly { path: string; what: string }[];
  /**
   * Where its work has been, from the record's `started_at` and each attachment's `since` and `changed_at`,
   * given-back rows included, which `attachments` leaves out (`trailOfRecord`).
   */
  trail?: SessionTrail;
  rows: readonly SessionRow[];
  turn: SessionTurn;
  /** When the last turn ended, already worded. Absent on a Session that has not had one. */
  lastTurn?: string;
  /** The same instant, for the list's "4m" and "2h". Absent where only the clock is known. */
  lastTurnAt?: string;
  /** What the agent is held on, while it is. */
  asked?: SessionAsk;
  /**
   * The last question the agent asked that nobody answered in time, while the person has said nothing since.
   * `id` is the ask's row. Not `asked`: that is a live call the agent is held on.
   */
  lapsed?: { id: string; questions: readonly SessionQuestion[] };
  /** What it is doing now and the agent's todo list. Absent is nothing to say. */
  activity?: SessionActivity;
  /** What it has built: the phase, a piece for each task, the road and the subagents at work, as Fleet derived them. Absent where Fleet observed nothing. */
  build?: SessionBuild;
  /** What the Session waits on the person for: the agent's list with Fleet's own items merged in. Absent or empty is nothing. */
  waitingFor?: readonly WaitingItem[];
  /** The agent says it has nothing left to work on, since when. `SessionRecord.done_since`. A message from the person clears it. */
  done?: { since: string };
  /**
   * What Fleet read the Session to be doing: working, stuck, replied, done or idle. **The one answer**
   * the Sessions list, the Cockpit's tiles, lanes and calls all draw. Absent from a Fleet that does not
   * say, and the Session is then read from its turn alone.
   */
  standing?: SessionStanding;
  /**
   * The first message its starting Session's agent wrote, which has not been sent. Nothing runs until the person sends it.
   * Absent once it is sent, or on a Session nobody's agent started.
   */
  unsent?: string;
  /** Tags chosen and not yet sent: they wait in the message box as chips. */
  pendingTags?: readonly SessionTag[];
  /** The model and effort the next turn runs on. Absent is Auto. */
  model?: string;
  /** The model Fleet heard the running process resolve, as its full id (`vendor-opus-5-5`). Absent until it has said. */
  resolvedModel?: string;
  effort?: string;
  /** The permission mode. **A Session runs in auto**, so absent is auto. */
  mode?: SessionMode;
  /** The plan, workflow, subagents and concerns the view draws round the thread. Absent draws the thread and its ledger as before. */
  work?: SessionWork;
};

/**
 * Something a message tags with `@`, or an issue or pull request with `#`, so the agent knows what
 * is meant: another Session, a Job, a pull request, a branch or an issue. A Job carries what the ledger
 * needs to hold it as one the Session is looking at.
 */
export type SessionTag = {
  kind: "session" | "job" | "pull_request" | "branch" | "issue";
  id: string;
  title: string;
  /** The forge's address of an issue or a pull request. */
  url?: string;
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
export type SessionAsk = {
  command: string;
  call?: string;
  offers?: readonly SessionAnswer[];
  /** The agent's own questions, where the ask is it asking the person rather than for a permission. */
  questions?: readonly SessionQuestion[];
};

/** One question the agent put to the person. */
export type SessionQuestion = {
  question: string;
  header: string;
  multi_select: boolean;
  options: readonly { label: string; description: string; preview?: string }[];
};

/** What was chosen for one question: option labels, and the person's own words where they chose Other. */
export type SessionQuestionAnswer = { question: string; chosen: string[] };

/** A subagent's thread, drawn as a Session's is. */
export type SubagentThread = { rows: SessionRow[]; finished: boolean; report?: string };

/** A file the panel read: its bytes and kind, or why it could not be shown. */
export type ArtifactRead =
  | { ok: true; bytes: Uint8Array; type: string }
  | { ok: false; why: "not_addressable" | "too_big" | "unreadable" | "binary"; limit?: number };

/** A pull request's page as read, or why it was not: Fleet's words for a refusal. */
export type PullRequestPageRead = { ok: true; page: PullRequestPage } | { ok: false; said: string };

/** One file of a pull request as read, or why it was not: Fleet's words for a refusal. */
export type PullRequestFileRead = { ok: true; file: PullRequestFilePatch } | { ok: false; said: string };

/** One uncommitted file of a held checkout as a unified diff against HEAD, or why it was not read: Fleet's words for a refusal. `cut` is set where Fleet sent only the start of a long change. */
export type CheckoutDiffRead = { ok: true; diff: string; cut?: true } | { ok: false; said: string };

/** A rect in the window's content, in CSS pixels. */
export type PageBounds = { x: number; y: number; width: number; height: number };

/**
 * What the window holds of Sessions, and the acts on them. **The mock's seam**
 * (`apps/desktop/src/renderer/src/sessions-draft.tsx`): a real Fleet gives none,
 * so a surface that reads this draws nothing and the rail row is left off. When
 * Fleet serves Sessions this becomes a read and these acts become commands.
 */
export type SessionsDraft = {
  get: () => readonly Session[];
  /**
   * The live Helm Session of the repository Helm's dock answers for, **whatever the rail picked**: the
   * list holds the picked repository's Sessions, and Helm may be pointed at another. Absent where the
   * list holds every Session, and the dock then finds it there.
   */
  helmOf?: (manifestId: string) => Session | undefined;
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
  /**
   * Starts a new Session as a copy of a Session's conversation up to and including one message, live or
   * not, and returns its id as `fork` does. The Session it came from is untouched. Absent where nothing
   * serves it, and selecting words in a message offers no Fork.
   */
  forkAt?: (id: string, at: string) => string | Promise<string | undefined>;
  /**
   * Ends a Session, if it is live, and starts a new one in another repository (`manifestId`) with the whole
   * conversation and the same title, and returns the new Session's id. The old one's slot, branch and
   * uncommitted files stay in the old repository. A Session in a terminal that is still listening is not
   * moved. Absent where nothing serves it, and the repository is not drawn as a control.
   */
  move?: (id: string, manifestId: string) => Promise<string | undefined>;
  /** One of the three ways out of a pilot. Absent with `pilot`. A refusal is said in `said`. */
  exit?: (jobId: string, exit: "submit" | "attest" | "supersede") => void;
  /**
   * Starts a blank Session and returns its id. It holds no slot and no branch until the agent writes.
   * **A promise on a real Fleet**, which names the Session; one that could not start says why in `said`.
   * `root` is the repository it starts in; absent, the picked one, or the only one Fleet serves.
   * `request` is the words the person dispatched with: Fleet reads them once to fill in the title, workflow,
   * model and effort, and the Session starts on the answer without the press waiting for it.
   */
  start: (tag?: SessionTag, root?: string, request?: string) => string | Promise<string | undefined>;
  /** Opens a Session's thread for reading. Absent where the thread is already held. */
  watch?: (id: string) => void;
  /**
   * A subagent's own thread as it stands, once read: the rows, whether it has finished, and its report.
   * Read again while it runs. Absent where nothing serves it, and the panel then shows the report alone.
   */
  subagent?: (id: string, subagentId: string) => Promise<SubagentThread | undefined>;
  /**
   * Writes a Session's retro and resolves when it is written, to whether it was. Absent where
   * nothing serves it, and the Retro press is left off rather than drawn dead.
   */
  retro?: (id: string) => Promise<boolean>;
  /** Ends a Session: the slot is parked and the row ends. Absent in the mock, which has no end. */
  close?: (id: string) => void;
  /** Ends the turn that is running; the Session rests and takes the next message. Absent in the mock's fixtures. */
  stop?: (id: string) => void;
  /**
   * Gives back the slots, branches, pull requests, Jobs and needs a Session holds, or hands them to the live Session
   * `to` of the same repository: the `rows` ticked, by the ledger's kind and target, or every one where absent. A
   * running turn is stopped first where a slot is among them, and the Session stays live. Resolves to what moved and
   * what stayed; a refusal is said as `stop`'s is and resolves to nothing. Absent where nothing serves it.
   */
  releaseHolds?: (id: string, to?: string, rows?: readonly { kind: string; target: string }[]) => Promise<HoldsReleased | undefined>;
  /** Names a Session, hosted or in a terminal. Absent where there is nothing to save it to. */
  rename?: (id: string, title: string) => void;
  /**
   * Opens, or opens again, a page a Session showed, in a window of Bridge's own. **A note captured in
   * it goes to the Session** as a message that wakes it. Absent where nothing serves it.
   */
  openWindow?: (id: string, url: string) => void;
  /** Opens a file a Session wrote. Absent in the mock, whose files are not on this machine. */
  openFile?: (id: string, path: string) => void;
  /** Reads a file the Session's ledger names, for the panel an Artifacts row opens. */
  readArtifact?: (id: string, path: string) => Promise<ArtifactRead>;
  /**
   * The web view a page or doc is shown in, over the panel's body. **Absent in the mock**, which
   * has no second view and draws a frame of its own page in its place.
   */
  page?: {
    show: (id: string, address: string, bounds: PageBounds) => void;
    move: (bounds: PageBounds) => void;
    hide: () => void;
    /** Esc pressed in the view, which has the keyboard while it is shown. Returns its remover. */
    onEscape: (on: () => void) => () => void;
  };
  /** Reads a pull request again, so its Checks are what the forge says now. Absent in the mock. */
  refresh?: (id: string, number: number) => void;
  /**
   * What the pull request panel's tabs and facts draw, read when the panel opens. Absent where
   * nothing serves it, and the panel then draws no tabs and no comment box.
   */
  pullRequestPage?: (id: string, number: number) => Promise<PullRequestPageRead>;
  /** One file of a pull request, read when a person opens it from the Files tab. Absent with `pullRequestPage`. */
  pullRequestFile?: (id: string, number: number, path: string) => Promise<PullRequestFileRead>;
  /**
   * A comment, an approval or a request for changes on a pull request. **Answers the page read
   * again after it**, or Fleet's words for a refusal. Absent with `pullRequestPage`.
   */
  writePullRequest?: (id: string, number: number, write: Pick<WritePullRequest, "kind" | "body">) => Promise<PullRequestPageRead>;
  /** What Fleet refused, in words, until the next act. Absent in the mock. */
  said?: () => string | undefined;
  /** What `@` offers beyond other Sessions: the Jobs. */
  taggable: () => readonly SessionTag[];
  /** Puts a Session's waiting tags where the person left them. */
  setTags: (id: string, tags: readonly SessionTag[]) => void;
  /**
   * A message from the person, with what they sent along. It takes a turn. **A promise on a real
   * Fleet**, which resolves to whether the message arrived; a refusal is already said as a toast.
   */
  send: (id: string, sent: SentMessage) => void | Promise<boolean>;
  /** Sets the model, effort and permission mode a Session's next turn runs on. `null` is Auto. */
  tune: (id: string, tuning: { model: string | null; effort: string | null; mode: SessionMode }) => void;
  /**
   * An act on one of a Session's pull requests. `review` dispatches a Job on the code review workflow against it.
   * **Settles when the act and the re-read after it are done**, with whether it was accepted, so the sheet can
   * draw the press as underway until then. A refusal is `said`, not thrown.
   */
  act: (id: string, number: number, act: PullRequestAct) => Promise<boolean>;
  /** Set where `act` dispatches the review itself, so the window does not propose one beside it. */
  reviews?: "fleet";
  /** The models and efforts a Session may be set to, as Dispatch offers them. */
  models: readonly string[];
  /** The model a Session follows while it names none, as Fleet's model list holds it. */
  defaultModel?: string;
  efforts: readonly string[];
  /** The skills and commands `/` offers. */
  commands: readonly SessionCommand[];
  /** Answers one waiting item: a numbered choice, words, or leave it to the agent (`best` thinks it through, `quick` takes the reasonable path). Absent where nothing serves it. */
  answerWaiting?: (id: string, itemId: string, answer: { choice?: number; text?: string; mode?: "best" | "quick" }) => void | Promise<unknown>;
  /**
   * Runs a command from a code block in a Session's thread, in the Session's working directory (its
   * slot if it has one), on a terminal of `size` cells, and reports the terminal's bytes and the exit as
   * they come. `write` types into the terminal and `resize` tells it the screen changed. The person's
   * own act: nothing is sent to the agent by the run, though Fleet tells the Session how it ended.
   * Absent where nothing serves it, and the block is left without Run. Fleet answers it with a Session
   * run (`start_session_run`, `observe_session_run`, `write_session_run`, `resize_session_run`,
   * `stop_session_run`).
   */
  run?: (
    id: string,
    command: string,
    on: (event: { kind: "out"; text: string } | { kind: "end"; code: number | null }) => void,
    size?: { cols: number; rows: number },
  ) => { stop: () => void; write?: (data: string) => void; resize?: (cols: number, rows: number) => void };
  /**
   * Whether a run is drawn as a terminal that takes keys, docked at the foot of the Session's view, as
   * opposed to lines of output under the block. Set where `run` answers with the terminal's bytes.
   */
  terminalRuns?: true;
  /**
   * As `run`, for a call about a repository's served checkout, which has no Session: the command runs in
   * the repository at `root`. A real Fleet is asked by the repository's Manifest (`catch_up_checkout`) and
   * runs the one pull it knows after reading the hold again, so `command` is what the card showed and is not
   * sent. Absent where nothing serves it, and the card offers Copy alone.
   */
  runInRepository?: (root: string, command: string, on: (event: { kind: "out"; text: string } | { kind: "end"; code: number | null }) => void) => { stop: () => void };
  /**
   * An act on a pull request no Session holds, such as a form edit's (`merge_by_hand`): the repository at `root` is named to
   * Fleet by its Manifest, which refuses a pull request that is not open or not that repository's. Resolves whether Fleet
   * took the act. Absent where nothing serves it, and the pull request is drawn without the act.
   */
  actInRepository?: (root: string, number: number, act: PullRequestAct) => Promise<boolean>;
  /**
   * One uncommitted file of a repository's held checkout, read by Fleet when a person opens it from "Your checkout".
   * Fleet refuses a path the hold does not list. Absent where nothing serves it, and the lines are plain text.
   */
  diffInRepository?: (root: string, path: string) => Promise<CheckoutDiffRead>;
  /**
   * Commits the uncommitted files of a repository's held checkout to a branch, opens a pull request with auto-merge on, and
   * answers its number, or `undefined` where Fleet refused (it said why). Fleet then carries the pull request on the hold
   * (`HeldCheckout.pull_request`). Absent where nothing serves it, and the card offers no such act.
   */
  sendCheckoutAsPullRequest?: (root: string) => Promise<{ number: number } | undefined>;
  /**
   * Types a slash command into a **terminal** Session's terminal, to run as typed, from inline code in
   * its thread. No output comes back: the command runs in the terminal. Absent where nothing serves it,
   * and in a Bridge-hosted Session, where there is no terminal. Fleet answers it as `tune_terminal`
   * does for `/model`: the Session's mod runs `held.commands` as typed (`tune_session` in
   * `session_host/serving.rs`).
   */
  command?: (id: string, text: string) => void;
  /** The acts on a Session's `work`. */
  work?: SessionWorkActs;
  /** Answers the permission a Session is held on. The mock offers two and names none. */
  answer: (id: string, answer?: SessionAnswer, answers?: SessionQuestionAnswer[]) => void;
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

/**
 * What the person is owed besides the reply and the done Fleet derives from the turn: the agent's own
 * items, an open card, a walk, a question asked in the window. `replied` and `done` are read from the
 * thread and the turn, so a surface that draws the thread (its last message, its done callout) leaves them out.
 */
export const owesOf = (session: Session): readonly WaitingItem[] =>
  (session.waitingFor ?? []).filter((one) => one.source !== "replied" && one.source !== "done");

/**
 * What the Session is doing: Fleet's reading where it gave one, else what the turn alone says. A Fleet
 * that predates `standing` never says `replied` or `stuck`, and a Session it called done still is.
 */
export const standingOf = (session: Session): SessionStanding =>
  session.standing ?? (session.turn.state === "working" ? "working" : session.done !== undefined ? "done" : "idle");

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
  // An ended Session keeps what it made on its ledger, but holds nothing.
  return sessions.find((session) => session.dead !== "ended" && session.attachments.some((one) => names(one, ref)));
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
