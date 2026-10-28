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
import type { SketchAnswer, SketchSettled } from "@armada/components/src/compositions/SessionThread/SketchRow";
import type { AdversarialReviewState } from "@armada/components/src/compositions/AdversarialReview/review-state";
import type { RunsPlace } from "@armada/components/src/compositions/RepositoryRuns/runs-places";
import type { CallKind, CheckBlame, DeliveryChoice, ProjectRowRef, DeliveryPicked, DraftSettled, LoadoutItemBody, LoadoutKind, LoadoutSource, LoadoutSummary, Notice, NoticeScope, PullRequestFilePatch, PullRequestPage, RequestBy, RequestHeader, SessionBuild, SessionLoadout, SessionStanding, ThoughtTarget, WaitingItem, WritePullRequest } from "@armada/protocol";
import type { McpServerAct, McpSignIn, SessionMcpServer, StepPhase, WatchSummary } from "@armada/protocol";
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
  | { kind: "slot"; slot: number }
  | { kind: "branch"; name: string; slot: number }
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
      /** The issue numbers the forge says it closes. Absent where it closes none. */
      closes?: number[];
      /** The branch it is based on. Another pull request's `branch` here stacks it on that one. Absent where Fleet has not read it. */
      base?: string;
    }
  /**
   * An issue the Session is working on. `number` is the forge's number as digits (`7`) or a tracker's key (`ABC-12`).
   * Its pull requests name a forge number in `closes`; a tracker's issue names them in `pull_requests`, from its row's detail.
   */
  | { kind: "issue"; number: string; title: string; address: string; state: "open" | "closed"; pull_requests?: number[]; /** The Job claimed it with `claim_issue`: work in hand, not one it filed or was told of. */ claimed?: true }
  | { kind: "studio"; id: string; title: string }
  /**
   * A sketch the agent published into the Session, or one the person shared with it.
   * **The agent's is a scene, and a second one with the same title replaces the first under the same `id`.**
   * Its thread card is derived from the `show_sketch` call's tool row (`sketchCallOf`), never a row of its own.
   * `against` is the sketch it is read as a change to, held whole so a later replacement does not move it.
   * `asked` is one put to the person to approve, and `settled` how he answered: **proposed, not on the wire**.
   */
  | { kind: "sketch"; id: string; title: string; by: "agent" | "you"; drawing: SessionSketch | Scene; against?: { title: string; drawing: SessionSketch | Scene }; asked?: true; settled?: SketchSettled }
  /** `agentType` is the type the spawning call named, such as `rust-engineer`. Absent where the ledger holds none. */
  | { kind: "subagent"; id: string; task: string; state: "running" | "done"; report?: string; agentType?: string; /** The Agent call that started it, which Fleet keys the Field's drone by. */ call?: string }
  /**
   * Something the Session's agent has running in the background: a shell command run with `run_in_background`,
   * or a Monitor watch. `task` is the call's description and `command` what it runs. `output` is the file
   * its output is written to, read as an artifact file is; absent where Fleet was not told it. `exit` is
   * the code a `done` or `failed` command ended on, absent where it was not told.
   * **Stopped is a command the agent or a person killed**, or one whose process went with the Session, and has no exit of its own.
   */
  | {
      kind: "background";
      id: string;
      form: "command" | "monitor";
      task: string;
      command: string;
      /** RFC 3339 instants. */
      started: string;
      ended?: string;
      state: "running" | "done" | "failed" | "stopped";
      exit?: number;
      output?: string;
    }
  /**
   * Something the Session made that a person would open: a page it published, a file it wrote outside
   * the code, or a Doc. `id` is the address of a page or document, or the path of a file.
   * **A code edit is never one**; those are Branches and Pull requests.
   */
  | {
      kind: "artifact";
      form: SessionArtifactForm;
      id: string;
      title: string;
      /** A file the agent put in front of the person with `show_file`: when, RFC 3339 with milliseconds. Each call has its own, so a second show of one path is told from the first. */
      shown?: string;
      by?: "agent";
      /** With `shown`: the Session wrote the file as well, and the ledger still says it wrote it. */
      written?: true;
      /** An HTML page the Job wrote as its document (form `document`): the path of each revision, oldest first. The newest is the one the Landing reads. `id` is the page's path in the worktree. */
      revisions?: readonly string[];
    }
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
  /**
   * A Session this one's agent started when asked, and the Session whose agent started this one. `id` is the other Session's.
   * `group` is the plan group it was handed from, which makes it a linked Session drawn on that group and in the Ledger's Linked section (`linkedOf`).
   * Every Session started from another stands under it in Work, group or none.
   */
  | { kind: "started_to"; id: string; group?: string }
  | { kind: "started_by"; id: string; group?: string }
  /** A web address the person or the agent wrote in the thread. Loopback addresses and the repository's own issue and pull request links are not kept. */
  | { kind: "link"; url: string; by: "you" | "agent" };

/** A `window` is a page the Session showed in a window of Bridge's own; `id` is its address. */
export type SessionArtifactForm = "page" | "file" | "image" | "doc" | "document" | "window";

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
export type WorkTask = { id: string; text: string; state: WorkTaskState; seen?: boolean; /** When the agent last changed this task's mark, RFC 3339. A mark the agent has not touched in a long while reads its age and dims. */ changedAt?: string; /** Fleet set the state itself, and this is its proof ("commit 1a2b3c4 names it"). */ marked?: string; /** In progress, and Fleet reads the work as elsewhere: why, and since when (RFC 3339). */ stale?: { why: string; since: string } };

/** Tasks one subagent takes, by the type of subagent. */
export type WorkGroup = { id: string; agent: string; title: string; /** What the group gives him once it is done, one line in his words. The card reads it before the title. */ gives?: string; /** The paths this group works in. */ paths: readonly string[]; /** The issue this group works, as digits or a tracker's key. Its title and state are those of the Session's `issue` attachment of that number. */ issue?: string; tasks: readonly WorkTask[] };

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

export type WorkPlan = { /** The version the agent recorded, which approving and sending changes name. */ version?: number; /** The plan in his words, a few lines of plain prose the card reads first. The groups are the technical detail under it. */ summary?: string; scope: readonly string[]; groups: readonly WorkGroup[]; approved: boolean; /** He said Not yet: the plan stays recorded, folded to a chip in the thread, and nothing waits on him for it. */ setAside?: true; /** What changed since the plan before this one. */ changes?: PlanChanges; /** The sketch drawn with the plan, if the agent drew one. */ sketch?: WorkSketch; /** The thread row that recorded it, which the approved plan settles after. */ after?: string };

/** A drone of this Session over the area it is working in: the Session's own, or a subagent of `agent` type. */
export type WorkDrone = { id: string; agent?: string; task?: string; path: string; doing: "read" | "write" | "run"; /** The file it is over, which the Field draws as a bar. */ file?: string; /** When Fleet saw it touch that file. */ at?: string };

/** A file of the repository as the Field draws it, a bar on the plot of its directory. `act` is what is being done to it now: made, changed or taken away. */
export type WorkFile = { path: string; act?: "write" | "edit" | "remove" };

/** A concern a judge raised at a step. It waits on the person; a judge never blocks. */
export type WorkConcern = { id: string; /** The step it was raised after, where a boundary judge raised it; the Judge tile's carry none. */ step?: string; judge?: string; text: string; /** The refusal in its parts, which `text` joins: what should be seen, what was seen instead, and what the difference does. Absent on a concern kept before they were. */ expected?: string; produced?: string; consequence?: string; /** What it cites: a file, and `:line` where it names one. */ cites?: readonly string[] };

/** A workflow a Session may follow: its steps in order. */
export type WorkflowChoice = { id: string; name: string; steps: readonly string[]; /** The repository that holds it. Absent is any. */ manifestId?: string };

/** One task on the agent's own list. The state is the plan task's, less the three a plan alone can say. */
export type AgentTask = { id: string; subject: string; state: "open" | "working" | "done" };

export type SessionWork = {
  /** What the proposer read the request to be. Changeable at any time; a change applies from the next turn. */
  proposal?: { /** The id of the workflow chosen. */ workflow: string | null; /** The proposer's call is out: the Session starts on its answer. */ reading?: boolean; /** What the proposer picked and the person has not changed: each is marked where it lives. */ picked?: readonly ("workflow" | "model" | "effort")[] };
  /** The workflow's steps and the index of the one the Session is at, where Fleet says. Absent is a custom Session, which has no rail; `at` absent draws the steps unlit. */
  workflow?: { name: string; steps: readonly string[]; at?: number; /** The phase of the step at `at`, as Fleet resolves it. */ phase?: StepPhase; /** When each step began, in order, as an RFC 3339 time. A step the Job went back to appears again. */ visits?: readonly { step: number; since: string }[] };
  /** Absent until the agent records its plan after planning with the person. */
  plan?: WorkPlan;
  /** The agent's own task list (its TaskCreate and TaskUpdate), in the order made. Progress lists it where there is no plan. */
  tasks?: readonly AgentTask[];
  field?: readonly WorkDrone[];
  /** The files the Field draws as bars on its plots. */
  files?: readonly WorkFile[];
  concerns?: readonly WorkConcern[];
  /** The latest adversarial review of the pending plan, each finding pinned to the group or task it is about on the card and in the plan in full. Absent is none run, which draws the press. */
  adversarial?: AdversarialReviewState;
  /** A judge is reading the work now. */
  judging?: true;
  /** The last reading's outcome: `concerns` 0 is how the tile says "No concerns". */
  judged?: { at: string; concerns: number };
  /** The owner's setting turned workflows off: the picker, the steps and the proposer's mark for a workflow are not drawn. */
  railOff?: true;
  /** What the workflow ends in, which the Landing panel draws: the workflow's `lands`, as Fleet resolves it. Absent is a pull request. */
  landing?: WorkLanding;
};

/** What a workflow lands in: a pull request and its merge, a spike's findings doc, a mod and its switch, a document, or a report with no pull request. */
export type WorkLanding = "pull_request" | "spike" | "mod" | "document" | "report";

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
  /** Not yet: the plan is set aside, its call on him goes, and the agent is told to keep talking and record no plan until he asks for one. Absent where nothing serves it, and Not yet is not drawn. */
  setAside?: (id: string) => void;
  /** Sends the concern to the Session as a message to the agent. */
  sendConcern: (id: string, concern: string) => void;
  dismissConcern: (id: string, concern: string) => void;
  /** The acts of an adversarial review of the plan: run it (again, where one was run), and send or dismiss a finding by its id. The review lands on the plan by Fleet's events. */
  adversarial?: { run: (id: string) => void; send: (id: string, finding: string) => void; dismiss: (id: string, finding: string) => void };
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
export type PullRequestAct = "ready" | "merge" | "auto_merge";

/**
 * `failed` is the required check red. `red` on the other two names a check that is red and not required:
 * on `pending` while the run goes on, on `passed` after the required checks passed.
 */
export type SessionChecks = { state: "pending"; red?: string } | { state: "passed"; red?: string } | { state: "failed"; failing: string };

export type SessionAttachmentKind = SessionAttachment["kind"];

/** Who said a row of a Session's thread. **A Session that wrote to this one is named**, never folded into "agent". */
export type SessionVoice =
  | { kind: "you" }
  | { kind: "agent" }
  | { kind: "session"; id: string; title: string };

/** What a followed Session did that its parent is told of. */
export type LinkedEvent = "needs" | "done" | "merged";

type SessionRowKinds =
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
      /** The Fleet notice this row was sent for, which a person can turn off. */
      notice?: Notice;
      /** A message of the person's that Fleet has not yet taken (`sending`) or did not (`failed`). Absent once it is Fleet's row. */
      delivery?: "sending" | "failed";
      /** What the row is drawn under while it changes from the message Bridge held to the row Fleet sent back, so it does not draw again. */
      stableKey?: string;
    }
  /**
   * Words typed in Helm's box that went to a Session and never to Helm. Only Helm's own Session
   * has these: Fleet keeps the row beside the ones the agent wrote, in the thread where it was sent.
   */
  | {
      id: string;
      at: string;
      kind: "sent";
      text: string;
      /** The Session the words went to, and the repository it is of as the picker names it. */
      target: ThoughtTarget & { repository: string };
      files?: readonly SentFile[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly SessionTag[];
      /** What the row is drawn under in place of the words Bridge held while they went, so it does not draw again. */
      stableKey?: string;
    }
  /** A tool call, mono. `fromPhone` is an ask the person answered on their phone. */
  | { id: string; at: string; kind: "tool"; text: string; fromPhone?: true }
  /** A command the person ran in the terminal, as typed. */
  | { id: string; at: string; kind: "command"; text: string }
  /** The summary the CLI wrote where it compacted the conversation: not the person's words. */
  | { id: string; at: string; kind: "compaction"; text: string }
  /** The agent disputed Fleet's notice `notice`: its reason, how it was spotted, and the review Job where one was started. */
  | { id: string; at: string; kind: "disputed"; notice: string; why: string; spotted: "called" | "read"; review?: string }
  /** The agent's first write: the slot leased and the branch cut, drawn in the thread where it happened. */
  | { id: string; at: string; kind: "lease"; slot: number; branch: string }
  /** The slot given back, drawn in the thread where it happened. */
  | { id: string; at: string; kind: "release"; slot: number; branch: string }
  /** A value the person changed through a panel, which the agent was told of: what it was and what it is now. */
  | { id: string; at: string; kind: "change"; what: string; from: string; to: string }
  /** A judge's concern he dismissed: one settled line, and the agent was told nothing. */
  | { id: string; at: string; kind: "concern"; step: string; text: string }
  /** The changes and notes he sent on the plan, one message with a line for each. */
  | { id: string; at: string; kind: "suggest"; lines: readonly SuggestLine[] }
  /** An `armada check` the agent ran in its slot: the Check's key, its run on the Checks page, where it stands, and where Fleet read it failed. */
  | { id: string; at: string; kind: "check"; name: string; run: number; state: "running" | "passed" | "failed"; target?: string; blame?: CheckBlame }
  /** A command the person ran from the thread or the band, which the Session was told: its exit (`code` is null where it was stopped) and the last lines it printed, as plain text. */
  | { id: string; at: string; kind: "ran"; command: string; code: number | null; tail: string }
  /**
   * An HTTP request sent from Requests, by him with this Session open or by its agent, and its answer. The request half is whole, to
   * reopen and send again; `status` is null where nothing answered and `error` says why.
   */
  | {
      id: string;
      at: string;
      kind: "requested";
      by: RequestBy;
      method: string;
      url: string;
      environment?: string;
      headers: readonly RequestHeader[];
      body?: string;
      status: number | null;
      ms: number;
      error?: string;
      response: string;
      truncated: boolean;
    }
  /** The agent saved a request into a collection in Requests. `collectionId` and `request` name it there. */
  | { id: string; at: string; kind: "saved"; name: string; collection: string; collectionId: string; request: string }
  /** The Session showed a page in a window: where in the thread it did, and the address to open again. `ended` is when the walk ended; absent while it is open. */
  | { id: string; at: string; kind: "window"; title: string; url: string; ended?: string; /** How many takes of the change the page shows. **A mock's field**: nothing in Fleet carries it yet. */ takes?: number }
  /** A draft the agent put to the person with `propose_draft`, whole as Fleet wrote it; `settled` once he answered it. */
  | { id: string; at: string; kind: "draft"; title: string; text: string; before?: string; settled?: DraftSettled }
  /** This Session's agent started a Session, or forked itself into one: the new Session, to open from the thread. */
  | { id: string; at: string; kind: "started"; session: { id: string; title: string }; fork: boolean }
  /**
   * A Session that waits on this one: it was dispatched to, its start condition was changed by this Session's agent or by the person,
   * or its condition was met and it started. `by` is who changed the condition. **A mock's row**: nothing in Fleet carries it yet.
   */
  | { id: string; at: string; kind: "waiter"; session: { id: string; title: string }; /** Written in the waiting Session's own thread: `session` is the one it waits on. */ waiting?: true; event: "waits" | "changed" | "started"; says?: string; from?: string; to?: string; by?: "you" | "agent" }
  /** A Session this one follows needs the person, finished, or merged its pull request. The agent was told in the same moment. */
  | { id: string; at: string; kind: "linked"; session: { id: string; title: string }; event: LinkedEvent }
  /** A retro being written for the Session, then the one that was: the latest frame of one press's row. */
  | RetroRowDrawn;

/** Every row's `at` is the RFC 3339 instant it happened. A mock's may be words, and the stamp then draws them as they came. */
export type SessionRow = SessionRowKinds;

/** One item of the agent's own TodoWrite list. The agent's report of itself; `SessionActivity.now` is what Fleet detected. */
export type SessionTodo = { text: string; state: "done" | "doing" | "todo" };

/**
 * What a Session is doing, from Fleet's PreToolUse gate: `now` is the last tool call as a short
 * action, and `todos` the agent's list where it keeps one. Todos are detail on top of `now`, never a
 * plan the Session is held to.
 */
export type SessionActivity = {
  now?: string;
  todos?: readonly SessionTodo[];
  /** Each subagent running now, by its Agent call: `label` is what it was sent to do, `now` its own last tool call as a short action. */
  subagents?: readonly { id: string; label: string; now?: string }[];
  /** The last sentence the agent wrote this turn, for a surface that draws what it said beside what it does. */
  said?: string;
};

/** A control pressed on a Session whose answer is still coming: Pause or Resume, Auto land, and Approve on the plan. */
export type PressedKey = "paused" | "autoLand" | "plan";

/** A turn running, or none. A message from another Session starts one, so `working` has no author. */
export type SessionTurn = { state: "idle" } | { state: "working"; wokenBy?: { id: string; title: string } };

/** One ledger row a release moved or left: its kind as the ledger names it (`slot`, `branch`, `pr`) and its target. */
export type HeldRow = {
  kind: string;
  target: string;
  /** A slot given back: the branch its work is on, and the WIP commit and files where something was uncommitted. */
  saved?: { branch: string; commit?: string; files: readonly string[] };
};

/** What a release gave back or handed over, and what stayed with why. `to` is the Session handed them. */
export type HoldsReleased = { to?: string; moved: readonly HeldRow[]; kept: readonly (HeldRow & { why: string })[] };

// ---- A Session that waits on another -------------------------------------------------------------------------
// Draft, mock-only: nothing in Fleet or the protocol carries it. A Session dispatched after another holds no slot and
// runs nothing until its condition is met; the Session it waits on is told, and may change the condition.

/** What starts a waiting Session: the other Session ends, a part of its plan is done, its pull request is pushed, or lands. */
export type StartWhen = "ends" | "plan" | "pushed" | "lands";

export type SessionWait = {
  /** The Session it waits on. */
  on: { id: string; title: string };
  when: StartWhen;
  /** With `plan`: the group or task of the other Session's plan, by id. */
  part?: { id: string; title: string };
  /** With `pushed` and `lands`: the pull request, once the other Session has named it. */
  pull?: { number: number; branch: string };
  /** Where it starts from: main, or the branch of the other Session's pull request. */
  base: StartBase;
  /** What it needs of the Session it waits on: the rest of the clause the brief names the wait in. Empty where the brief names none. */
  needs: string;
  /** The words it was dispatched with, which it starts on. */
  brief: string;
  /** Who last set the condition. */
  by: "you" | "agent";
};

export type StartBase = "main" | "branch";

/**
 * What the proposer reads of a brief that tags a Session: who it waits on, when it starts, where from, and what it needs. **A mock's
 * stand-in**: the mock's `reads-waits.ts` reads the words by rule, and nothing in Fleet reads a wait yet.
 */
export type WaitReading = {
  on: { id: string; title: string };
  when: StartWhen;
  part?: { id: string; title: string };
  base: StartBase;
  needs: string;
  /** What the proposer would name the Session. Absent where it reads no name, as a Fleet's reading of the wait does. */
  title?: string;
  /** The brief tags the Session and names no condition: the new Session does not wait on it, and the tag is a reference. */
  plain?: boolean;
};

export type Session = {
  id: string;
  /** What other sessions call it, `s-` and the first eight characters of the id. Drawn where the id would be. */
  address?: string;
  /**
   * Nothing more can be said to it: it `ended`, or it runs in a terminal whose mod has stopped asking
   * (`quiet`). Such a Session offers Fork in place of a message box, and owns nothing.
   */
  dead?: "ended" | "quiet";
  /**
   * A Job that asked to carry on in a new one and was told yes. `starting` while the new Job comes up, `failed` when it did
   * not: the Job is exactly as it was and keeps its row. Absent otherwise.
   */
  handoff?: "starting" | "failed";
  /** Why the new Job did not start, as Fleet records it in `detail.reason` on the `handoff_failed` row. Present with `handoff: "failed"`. */
  handoffReason?: string;
  /** The Job that took this one's place, by id. This one is hidden behind it: left out of every list, and reached from the new Job's row. */
  replacedBy?: string;
  /** The Job this one took the place of, by id. Its turns stand above this one's, over a plain divider. */
  replaces?: string;
  /** Parked by the person: the running turn stopped, its work committed to its branch and its slot given back. It stays in the list; Resume, or a message, takes a slot again. */
  paused?: true;
  /** Auto land is on: Fleet turns on the forge's auto-merge for each pull request this Session holds, and merges it on a green `ci`. Hosted Sessions only. */
  autoLand?: true;
  /** What was pressed and Fleet has not answered: `paused`, `autoLand` and `plan` are drawn as pressed already, with a going mark on the control (`pressed.ts`). Never Fleet's. */
  pressed?: readonly PressedKey[];
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
  /** What its agent knows and was told, counted, and what the latest turn used. Absent until the Session reports one. */
  loadout?: LoadoutSummary;
  /**
   * Helm's own Session for a repository: hosted like any other, pinned to Helm's dock, one per
   * repository. The Sessions list marks it and opening it opens the dock, not a page of its own.
   * It runs in auto, so `mode` is never offered for it.
   */
  helm?: true;
  /** A Mod Job: started from Mod, for modding Armada itself. */
  mod?: true;
  /** Absent until the first turn has named it; a blank Session is known by its id alone. */
  title?: string;
  /** The repository it works in, by Manifest id: what `#` reads issues and pull requests of. */
  manifestId?: string;
  /** The directory it was started in, `SessionRecord.cwd`. A report about it names where its transcript is kept. */
  cwd?: string;
  attachments: readonly SessionAttachment[];
  /**
   * Every version of every attachment, oldest first; `at` is an RFC 3339 instant and each entry is the attachment as it stood from then.
   * `from` is what the version followed, which Fleet works out: `message` is the thread row id of the person's latest message before it, and
   * `task` the id of the plan task in progress when it was written. Either may be absent.
   * Not on the record: the Artifacts graph reads it (`SessionsDraft.history`) when it opens. Absent until read, or where it could not be:
   * the graph then draws one mark to an attachment, in the order it is held.
   */
  versions?: readonly { at: string; attachment: SessionAttachment; from?: { message?: string; task?: string } }[];
  /**
   * Where its work has been, from the record's `started_at` and each attachment's `since` and `changed_at`,
   * given-back rows included, which `attachments` leaves out (`trailOfRecord`).
   */
  trail?: SessionTrail;
  rows: readonly SessionRow[];
  /** Absent once the thread is read. `unread`: no rows held yet. `refreshing`: rows held and being read again. */
  transcript?: "unread" | "refreshing";
  turn: SessionTurn;
  /** When the last turn ended, an RFC 3339 instant. Absent on a Session that has not had one. */
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
  /** The running account of a turn in his words, answering the message row `for`: what is done, what it is on now and what is next. No file or tool names. */
  account?: { for: string; done: readonly string[]; now?: string; next?: string; /** The id of the approved plan's group the now line belongs to; named in that line and marked on its card. */ group?: string; at: string };
  /** The page or sketch the agent is building, drawn live beside the account. */
  live?: { url: string; title: string };
  /** What it has built: the phase, a piece for each task, the road and the subagents at work, as Fleet derived them. Absent where Fleet observed nothing. */
  build?: SessionBuild;
  /** What the Session waits on the person for: the agent's list with Fleet's own items merged in. Absent or empty is nothing. */
  waitingFor?: readonly WaitingItem[];
  /** The follow-ups the agent kept for itself and has not settled, oldest first. `SessionRecord.later`; `id` is what a Drop names. */
  later?: readonly { id: string; text: string; at: string; /** The person marked it Deferred: Fleet was right to keep it, and its thumbs are gone. */ confirmed?: boolean }[];
  /** Its watches, live and ended, the newest first, as `watches.changed` carries them. Absent or empty is none. */
  watches?: readonly WatchSummary[];
  /** The name of the live watch that alone keeps it open. `SessionRecord.held_open_by`. */
  heldOpenBy?: string;
  /** The agent says it has nothing left to work on, since when. `SessionRecord.done_since`. A message from the person clears it. */
  done?: { since: string };
  /**
   * What Fleet read the Session to be doing: working, stuck, replied, done or idle. **The one answer**
   * the Sessions list, the Cockpit's tiles, lanes and calls all draw. Absent from a Fleet that does not
   * say, and the Session is then read from its turn alone.
   */
  standing?: SessionStanding;
  /**
   * Held on its start card: the proposer has read the request or is reading it, and no agent moves until Start. `asksDelivery` is
   * whether the Job's workflow has a landing step, so a choice is owed. `picked` is the choice already standing: one his request named, else his usual for this
   * kind in this repository, else Fleet's `default`. Absent once the Job has begun.
   */
  start?: { asksDelivery: boolean; picked?: DeliveryPicked; /** The first message the agent will read, held in the composer for him to edit. Absent until one is kept for the Job. */ message?: string };
  /** Tags chosen and not yet sent: they wait in the message box as chips. */
  pendingTags?: readonly SessionTag[];
  /** The model and effort the next turn runs on. Absent is Auto. */
  model?: string;
  /** The model Fleet heard the running process resolve, as its full id (`vendor-opus-5-5`). Absent until it has said. */
  resolvedModel?: string;
  effort?: string;
  /** The permission mode. **A Session runs in auto**, so absent is auto. */
  mode?: SessionMode;
  /** How the Job was started, read at the top of its story under the Brief. Absent falls back to what the Job holds now. A row left out is not drawn. */
  started?: { workflow?: string; delivery?: DeliveryChoice; model?: string; effort?: string; mode?: SessionMode; /** The Job it was started from. */ by?: { job: string; title: string } };
  /** The plan, workflow, subagents and concerns the view draws round the thread. Absent draws the thread and its ledger as before. */
  work?: SessionWork;
  /** Dispatched after another Session and not started: it holds no slot and runs nothing until the condition is met. Absent once it has started. */
  waits?: SessionWait;
};

/**
 * How a repository runs, read from commands Fleet saw exit and pass in its Sessions, never from what an agent said. Fleet keeps a command that passed
 * under the workspace it ran in, in the place chosen for the repository; a guess that failed is never kept.
 */
export type RepositoryRuns = {
  repository: string;
  /** Where it was seen. */
  source: string;
  rows: readonly { id: string; job: string; workspace?: string; command?: string; port?: number; waiting?: string }[];
  /** Where it is kept. */
  place: RunsPlace;
  /** Fleet's words when it would not answer, in place of the rows. */
  said?: string;
};

/** A folder the agent CLI kept sessions for under its own projects directory: when the last one ran, and whether the folder is a git repository. */
export type ClaudeFolder = { path: string; lastAt: string; git: boolean };

/**
 * Something a message tags with `@`, or an issue or pull request with `#`, so the agent knows what
 * is meant: another Session, a pull request, a branch or an issue.
 */
export type SessionTag = {
  kind: "session" | "pull_request" | "branch" | "issue";
  id: string;
  title: string;
  /** The forge's address of an issue or a pull request. */
  url?: string;
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
export type SessionAnswer = "allow_once" | "allow_and_remember" | "refuse" | "always_allow" | "always_block" | "allow_for_session" | "allow_like_this_for_session";

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
  /** The absolute path past the session's slots it asked about, which the "always" answers name a folder of. */
  reaches?: string;
  /** Whether the call reads that path or writes to it. */
  reachesTo?: "read" | "write";
  /** The Bash call's program and script, which "Allow calls like this for this job" allows a width of. */
  kind?: CallKind;
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
export type SubagentThread = {
  rows: SessionRow[];
  finished: boolean;
  report?: string;
  /** The model id it runs on, as Fleet read it. Absent until it has answered. */
  model?: string;
  /** What its last answer had in its window. Absent until it has answered. */
  context?: { tokens: number; window: number };
};

/** A file the panel read: its bytes and kind, or why it could not be shown. */
export type ArtifactRead =
  | { ok: true; bytes: Uint8Array; type: string }
  | { ok: false; why: "not_addressable" | "too_big" | "unreadable" | "binary"; limit?: number };

/** One Loadout row, as `read_loadout_item` is asked for it. A use's `skill`, `tool` or `subagent` is the same kind. */
export type LoadoutItemAsk = { kind: LoadoutKind; name: string; source: LoadoutSource; from?: string };

export type { LoadoutItemBody };

/** A hosted Session's MCP servers as read, or Fleet's words for why not. */
export type McpRead = { ok: true; servers: readonly SessionMcpServer[] } | { ok: false; said: string };

/** The servers read again after an act, and for a sign-in the page to open in the person's browser; or Fleet's words for a refusal. */
export type McpActed = { ok: true; servers: readonly SessionMcpServer[]; signIn?: McpSignIn } | { ok: false; said: string };

/** A pull request's page as read, or why it was not: Fleet's words for a refusal. */
export type PullRequestPageRead = { ok: true; page: PullRequestPage } | { ok: false; said: string };

/** One file of a pull request as read, or why it was not: Fleet's words for a refusal. */
export type PullRequestFileRead = { ok: true; file: PullRequestFilePatch } | { ok: false; said: string };

/**
 * One file of a write clash: each side's change to it as a unified diff from where the two split, or why it was not read. `ours`
 * is the Session's own. Mock only until Fleet serves it.
 */
export type ClashFileRead = { ok: true; ours: string; theirs: string } | { ok: false; said: string };

/**
 * One file a Job has been seen writing, as its change since the base: the diff and the stamp to send back as `seen`. `unchanged` is
 * set where the stamp sent was this one, and the diff is then empty. A refusal carries no words: the lens that asked draws nothing.
 */
export type FileChangeRead = { ok: true; stamp: string; diff: string; cut?: true; unchanged?: true } | { ok: false; noRoute?: true };

/** One uncommitted file of a held checkout as a unified diff against HEAD, or why it was not read: Fleet's words for a refusal. `cut` is set where Fleet sent only the start of a long change. */
export type CheckoutDiffRead = { ok: true; diff: string; cut?: true } | { ok: false; said: string };

/** A rect in the window's content, in CSS pixels. */
export type PageBounds = { x: number; y: number; width: number; height: number };

/** A picture of a page main holds out of sight, as PNG bytes: the live view draws it scaled, and it is made again as the page changes. */
export type PageShot = { address: string; png: Uint8Array };

/** An idea saved to start later: the words, the repository it is for, and the model and effort picked for it. */
export type QueuedIdea = {
  id: string;
  text: string;
  /** The repository's root, as `start` takes it, and its name as the rail draws it. */
  root: string;
  repository: string;
  model?: string;
  effort?: string;
};

/** What a run says: the terminal's bytes, then how it ended. A run with no exit code was ended by Stop (`stopped` true) or by a signal (false). */
export type RunEvent = { kind: "out"; text: string } | { kind: "end"; code: number } | { kind: "end"; code: null; stopped: boolean };

/**
 * What the window holds of Sessions, and the acts on them. **The mock's seam**
 * (`apps/desktop/src/renderer/src/sessions-draft.tsx`): a real Fleet gives none,
 * so a surface that reads this draws nothing and the rail row is left off. When
 * Fleet serves Sessions this becomes a read and these acts become commands.
 */
export type SessionsDraft = {
  get: () => readonly Session[];
  /**
   * The Sessions of every repository, **whatever the rail picked**: the palette jumps to work in any of them. Absent
   * where `get` already holds every one, and then `get` is what it reads.
   */
  everywhere?: () => readonly Session[];
  /**
   * The live Helm Session of the repository Helm's dock answers for, **whatever the rail picked**: the
   * list holds the picked repository's Sessions, and Helm may be pointed at another. Absent where the
   * list holds every Session, and the dock then finds it there.
   */
  helmOf?: (manifestId: string) => Session | undefined;
  subscribe: (onChange: () => void) => () => void;
  /**
   * Hands a plan group to a new Session this one follows, and returns its id: **a promise on a real Fleet**, which starts the Session and sends
   * it the group's tasks. Absent where nothing serves it, and a group carries no act to hand it off.
   */
  handOff?: (id: string, group: string) => string | Promise<string | undefined> | undefined;
  /**
   * Starts a new Session as a copy of a dead one's conversation and returns its id.
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
  /** Turns a Fleet notice off for a Job, its repository or an hour, by `set_notice_mute`. A refusal is a toast. Absent where nothing serves it, and a notice has no menu. */
  muteNotice?: (notice: Notice, scope: NoticeScope) => Promise<void>;
  /**
   * Starts a blank Session and returns its id. It holds no slot and no branch until the agent writes.
   * **A promise on a real Fleet**, which names the Session; one that could not start says why in `said`.
   * `root` is the repository it starts in; absent, the picked one, or the only one Fleet serves.
   * `request` is the words the person dispatched with: Fleet reads them once to fill in the title, workflow,
   * model and effort, and the Session starts on the answer without the press waiting for it.
   * `tuning` is a model or effort the person picked in Dispatch; it wins, and Fleet fills in only what is left out.
   * `after` is a Session this one waits on and when it starts: it then holds its words and runs nothing until the condition is met.
   * `held` starts it on its start card: the words are read and nothing runs until `begin`.
   * `projectRows` are the rows of the repository's projects it starts with: each written task is held by the Job.
   */
  start: (tag?: SessionTag, root?: string, request?: string, tuning?: { model?: string; effort?: string }, after?: Pick<SessionWait, "on" | "when" | "part" | "base" | "needs">, held?: boolean, projectRows?: readonly ProjectRowRef[]) => string | Promise<string | undefined>;
  /**
   * Begins a Session held on its start card: the agent starts, and `delivery` is how the work lands where the Job's workflow has a landing
   * step. `message` is the first message as he left it on the card. Absent where nothing serves it, and the card offers no Start.
   */
  begin?: (id: string, delivery?: DeliveryChoice, message?: string) => void | Promise<boolean>;
  /** Reads a brief for the wait it names: absent where nothing reads one, and Dispatch draws no sentence. */
  readsWait?: (words: string) => WaitReading | undefined;
  /**
   * The reading of a brief once the proposer has answered for it, which a send waits for so a Session is not started on the defaults
   * while the real reading is out. Absent where `readsWait` answers at once.
   */
  settlesWait?: (words: string) => Promise<WaitReading | undefined>;
  /**
   * Changes when a waiting Session starts, and from where. The Session it waits on is told. Absent where nothing serves it, and the
   * waiting Session's page offers no condition to change.
   */
  startWhen?: (id: string, when: StartWhen, part?: { id: string; title: string }) => void;
  /** Starts a waiting Session now, whatever its condition. The Session it waits on is told. Absent with `startWhen`. */
  startNow?: (id: string) => void;
  /** Ideas saved to start later, oldest first. Absent where nothing serves them, and Dispatch offers no Queue for later. */
  queued?: () => readonly QueuedIdea[];
  /** Saves an idea without starting it. Absent with `queued`. */
  queue?: (idea: Omit<QueuedIdea, "id">) => void;
  /** Starts the Session a queued idea describes, takes the idea off the queue, and returns the Session's id as `start` does. Absent with `queued`. */
  startQueued?: (id: string) => string | Promise<string | undefined>;
  /** Takes an idea off the queue without starting it. Absent with `queued`. */
  dropQueued?: (id: string) => void;
  /** Opens a Session's thread for reading. Absent where the thread is already held. */
  watch?: (id: string) => void;
  /**
   * A subagent's own thread as it stands, once read: the rows, whether it has finished, and its report.
   * Read again while it runs. Absent where nothing serves it, and the panel then shows the report alone.
   */
  subagent?: (id: string, subagentId: string) => Promise<SubagentThread | undefined>;
  /** A Session's whole Loadout, once read; undefined where it could not be. Absent where nothing serves it. */
  loadout?: (id: string) => Promise<SessionLoadout | undefined>;
  /** A Session's attachment history, once read; undefined where it could not be. Absent where nothing serves it. */
  history?: (id: string) => Promise<NonNullable<Session["versions"]> | undefined>;
  /** What one Loadout item says, once read; undefined where it could not be. Absent where nothing serves it. */
  loadoutItem?: (id: string, item: LoadoutItemAsk) => Promise<LoadoutItemBody | undefined>;
  /**
   * A hosted Session's MCP servers as its CLI reports them now. The CLI pushes no change, so this is read again while the
   * Loadout is open. Absent where nothing serves it, and the Loadout draws its servers from the agent's own report.
   */
  mcp?: (id: string) => Promise<McpRead>;
  /** Acts on one MCP server of a hosted Session. A Reconnect or a Turn on answers once the CLI has tried to connect, up to 90s. Absent with `mcp`. */
  actOnMcp?: (id: string, server: string, act: McpServerAct) => Promise<McpActed>;
  /**
   * Writes a Session's retro and resolves when it is written, to whether it was. Absent where
   * nothing serves it, and the Retro press is left off rather than drawn dead.
   */
  retro?: (id: string) => Promise<boolean>;
  /** Ends a Session: the slot is parked and the row ends. Absent in the mock except where a scenario scripts the end. */
  close?: (id: string) => void;
  /** Ends the turn that is running; the Session rests and takes the next message. Absent in the mock's fixtures. */
  stop?: (id: string) => void;
  /** The person drops a follow-up the Session kept (`later` is its id in `Session.later`), and Fleet takes it off the list; `misread` is thumbs down, Not deferred, which says Fleet should not have kept it. */
  dropLater?: (id: string, later: string, misread?: boolean) => void;
  /** The person marks a kept follow-up Deferred (thumbs up): it stays on the list without its thumbs. */
  confirmLater?: (id: string, later: string) => void;
  /** The person's Stop on one of a Session's watches (`watch` is its id): Fleet ends it `stopped`. */
  stopWatch?: (id: string, watch: string) => void;
  /** The person's delete on one of a Session's ended watches (`watch` is its id): Fleet forgets it and all kept of it. */
  deleteWatch?: (id: string, watch: string) => void;
  /** A failed reading's log (`log` is the path its reading carries), read whole: its text, or why it could not be read. */
  readWatchLog?: (watch: string, log: string) => Promise<{ ok: true; text: string } | { ok: false; said: string }>;
  /** Runs a failed hand off again for Job `id`. */
  retryHandoff?: (id: string) => void;
  /**
   * Gives back the slots, branches, pull requests and needs a Session holds, or hands them to the live Session
   * `to` of the same repository: the `rows` ticked, by the ledger's kind and target, or every one where absent. A
   * running turn is stopped first where a slot is among them, and the Session stays live. Resolves to what moved and
   * what stayed; a refusal is said as `stop`'s is and resolves to nothing. Absent where nothing serves it.
   */
  releaseHolds?: (id: string, to?: string, rows?: readonly { kind: string; target: string }[]) => Promise<HoldsReleased | undefined>;
  /**
   * Stops the running turn, commits the work to the Session's branch and gives its slot back; the Session stays listed as paused.
   * Resolves false when Fleet refuses, and says why in `said`. Absent where nothing serves it.
   */
  pause?: (id: string) => Promise<boolean>;
  /**
   * Turns Auto land on or off. Resolves true once the flag changed, which is also where the forge refused a pull request: that
   * refusal is said as `stop`'s is, and the Session's `autoLand` follows from Fleet's record. False where nothing changed. Absent where nothing serves it.
   */
  autoLand?: (id: string, on: boolean) => Promise<boolean>;
  /** Lifts a pause: the Session takes a slot again. A message to a paused Session does the same. Absent with `pause`. */
  resume?: (id: string) => Promise<boolean>;
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
    /**
     * A page drawn small, as a picture main takes of it and takes again as it changes, where a web view over the panel cannot be scaled.
     * `watch` starts them for `address`, `stop` ends them, and `onShot` hears each one. **Absent in the mock**, which scales a frame of its own page.
     */
    snapshot?: {
      watch: (id: string, address: string) => void;
      stop: () => void;
      onShot: (on: (shot: PageShot) => void) => () => void;
    };
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
  /** One file of a write clash between the Session and `other`, read when a person opens it from Bays. */
  clashFile?: (id: string, other: { kind: "session" | "main"; id: string }, path: string) => Promise<ClashFileRead>;
  /** One file the Session has been seen writing, for its Field's lens. `seen` is the stamp of the change already held. Absent where nothing serves it. */
  fileChange?: (id: string, path: string, seen?: string) => Promise<FileChangeRead>;
  /**
   * A comment, an approval or a request for changes on a pull request. **Answers the page read
   * again after it**, or Fleet's words for a refusal. Absent with `pullRequestPage`.
   */
  writePullRequest?: (id: string, number: number, write: Pick<WritePullRequest, "kind" | "body">) => Promise<PullRequestPageRead>;
  /** What Fleet refused, in words, until the next act. Absent in the mock. */
  said?: () => string | undefined;
  /** Puts a Session's waiting tags where the person left them. */
  setTags: (id: string, tags: readonly SessionTag[]) => void;
  /**
   * A message from the person, with what they sent along. It takes a turn. **A promise on a real
   * Fleet**, which resolves to whether the message arrived; a refusal is already said as a toast.
   */
  send: (id: string, sent: SentMessage) => void | Promise<boolean>;
  /** Sends a message that did not go again, by the id of the row that holds it. Present where `send` holds a message while it goes. */
  retry?: (id: string, rowId: string) => void;
  /** Sets the model, effort and permission mode a Session's next turn runs on. `null` is Auto. */
  tune: (id: string, tuning: { model: string | null; effort: string | null; mode: SessionMode }) => void;
  /**
   * An act on one of a Session's pull requests. `review` starts a Session on the code review workflow against it.
   * **Settles when the act and the re-read after it are done**, with whether it was accepted, so the sheet can
   * draw the press as underway until then. A refusal is `said`, not thrown.
   */
  act: (id: string, number: number, act: PullRequestAct) => Promise<boolean>;
  /** The models and efforts a Session may be set to, as Dispatch offers them. */
  models: readonly string[];
  /** The model a Session follows while it names none, as Fleet's model list holds it. */
  defaultModel?: string;
  efforts: readonly string[];
  /** The skills and commands `/` offers. */
  commands: readonly SessionCommand[];
  /**
   * Answers one waiting item: a numbered choice, words, or leave it to the agent (`best` thinks it through, `quick` takes the
   * reasonable path). `fixed` approves a draft with his own text. Absent where nothing serves it.
   */
  answerWaiting?: (id: string, itemId: string, answer: { choice?: number; text?: string; mode?: "best" | "quick"; fixed?: string }) => void | Promise<unknown>;
  /** Answers a sketch put to him, by its ledger id: approved, or sent back with a note. **Proposed, not on the wire**: only the mock serves it. */
  answerSketch?: (id: string, sketch: string, answer: SketchAnswer) => void | Promise<unknown>;
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
    on: (event: RunEvent) => void,
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
  runInRepository?: (root: string, command: string, on: (event: RunEvent) => void) => { stop: () => void };
  /**
   * An act on a pull request no Session holds, such as a form edit's (`merge_by_hand`): the repository at `root` is named to
   * Fleet by its Manifest, which refuses a pull request that is not open or not that repository's. Resolves whether Fleet
   * took the act. Absent where nothing serves it, and the pull request is drawn without the act.
   */
  actInRepository?: (root: string, number: number, act: PullRequestAct) => Promise<boolean>;
  /**
   * Has Fleet read one pull request from the forge now, by the repository's Manifest. What changed reaches the window as Fleet's
   * events, so nothing is folded from the answer; a refusal, or no Manifest, is a toast. Absent where nothing serves it, and a
   * pull request chip has no menu.
   */
  refreshPullRequest?: (manifestId: string | undefined, number: number) => Promise<void>;
  /**
   * One uncommitted file of a repository's held checkout, read by Fleet when a person opens it from "Your checkout".
   * Fleet refuses a path the hold does not list. Absent where nothing serves it, and the lines are plain text.
   */
  diffInRepository?: (root: string, path: string) => Promise<CheckoutDiffRead>;
  /**
   * Commits the uncommitted files of a repository's held checkout to a branch, opens a pull request with auto-merge on, and
   * answers once Fleet accepted the send (the pull request's number is not known then), or `undefined` where Fleet refused (it said why). Fleet then carries the pull request on the hold
   * (`HeldCheckout.pull_request`). Absent where nothing serves it, and the card offers no such act.
   */
  sendCheckoutAsPullRequest?: (root: string) => Promise<object | undefined>;
  /**
   * Calls `on` when a send from this repository's held checkout fails after Fleet accepted it (`act.failed`), and answers how to stop
   * listening. Absent where nothing serves it; the card then waits on the hold alone.
   */
  checkoutSendFailed?: (root: string, on: () => void) => () => void;
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
  answer: (id: string, answer?: SessionAnswer, answers?: SessionQuestionAnswer[], folder?: string, kind?: CallKind) => void;
};

/** Anything a chip anywhere in Bridge can name. A chip asks who owns it. */
export type ChipRef =
  | { kind: "branch"; name: string }
  | { kind: "pull_request"; number: number }
  | { kind: "slot"; slot: number };

/** Whether a Session has leased anything yet. A blank one has no slot and no branch. */
export function isBlank(session: Session): boolean {
  return session.blank ?? (session.attachments.length === 0 && session.rows.length === 0);
}

/** The Sessions a parent follows: those its plan groups were handed to, with the group each came from. */
export function linkedOf(sessions: readonly Session[], parent: Session): { session: Session; group: string }[] {
  return parent.attachments.flatMap((one) => {
    if (one.kind !== "started_to" || one.group === undefined) return [];
    const session = sessions.find((other) => other.id === one.id);
    return session === undefined ? [] : [{ session, group: one.group }];
  });
}

/** An issue as a person reads it: a forge number with its `#` (`#7`), a tracker's key as it is (`ABC-12`). */
export const issueLabel = (issue: string): string => (/^[0-9]+$/.test(issue) ? `#${issue}` : issue);

export function attachmentsOf<K extends SessionAttachmentKind>(
  session: Session,
  kind: K,
): Extract<SessionAttachment, { kind: K }>[] {
  return session.attachments.filter((one): one is Extract<SessionAttachment, { kind: K }> => one.kind === kind);
}

/**
 * What the person is owed besides the done Fleet derives from the turn: the agent's own items, an open
 * card, a walk, a question asked in the window, and a `replied` item, which is a reply that asks him
 * something (its text is the line that asks). A reply that only tells is a `standing`, never an item.
 * `done` is read from the turn, so a surface that draws the thread (its done callout) leaves it out.
 */
export const owesOf = (session: Session): readonly WaitingItem[] =>
  (session.waitingFor ?? []).filter((one) => one.source !== "done");

/** Whether Fleet is putting the plan to him. A pending plan is recorded long before it is asked, and it is drawn only while this holds. */
export const planAsked = (session: Session): boolean => owesOf(session).some((one) => one.source === "plan");

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
        (attachment.kind === "pull_request" && attachment.branch === ref.name)
      );
    case "pull_request":
      return attachment.kind === "pull_request" && attachment.number === ref.number;
    case "slot":
      return attachment.kind === "slot" && attachment.slot === ref.slot;
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
 * The Sessions a query finds, by PR number, branch, slot or title.
 *
 * **`#1843` is a pull request and nothing else**, as a leading `#` is everywhere
 * in Armada. `slot 3` and `pr 52` say their kind; a bare number is either, a PR by prefix of its number.
 */
export function sessionsMatching(sessions: readonly Session[], query: string): SessionHit[] {
  const said = query.trim().toLowerCase();
  if (said === "") return sessions.map((session) => ({ session, matched: "id" }));
  const hash = said.startsWith("#");
  const kinded = /^(slot|pr)\s*#?\s*(\S+)$/.exec(said);
  const word = hash ? said.slice(1) : kinded !== null ? kinded[2]! : said;
  const only: SessionAttachmentKind | undefined = hash
    ? "pull_request"
    : kinded === null
      ? undefined
      : ({ slot: "slot", pr: "pull_request" } as const)[kinded[1] as "slot" | "pr"];

  const hits: SessionHit[] = [];
  for (const session of sessions) {
    const attached = session.attachments.find((one) => {
      if (only !== undefined && one.kind !== only) return false;
      switch (one.kind) {
        case "pull_request":
          return String(one.number).startsWith(word.replace(/^#/, ""));
        case "branch":
          return only === undefined && one.name.toLowerCase().includes(word);
        case "slot":
          return NUMBER.test(word) && one.slot === Number(word);
        default:
          return false;
      }
    });
    if (attached !== undefined) hits.push({ session, matched: attached });
    else if (only === undefined && session.title?.toLowerCase().includes(word)) hits.push({ session, matched: "title" });
  }
  return hits;
}
