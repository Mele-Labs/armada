// A Session: a raw, open-ended conversation with a hosted agent, with Armada's
// features hooked in. Draft, mock-only: nothing in Fleet or the protocol
// carries it yet, so every field here is one Fleet would owe. The owner's ask,
// 6 Oct 2026.
//
// A Session is a thread of its own that starts with
// no slot and no branch, leases a worktree slot on the agent's first write,
// and from then on accumulates what it touches.

import type { ContextBreakdown } from "../../components/context-breakdown";
import type {
  LoadoutSummary,
  SessionBuild,
  WaitingItem,
  HeldOpenBy,
  SessionStanding,
  DeliveryPicked,
  DeliveryChoice,
  JudgeReading,
  CallKind,
  StepPhase,
  ThoughtTarget,
  ToolSubagent,
  CheckBlame,
  RequestBy,
  RequestHeader,
  DraftSettled,
  PlanGate,
  GateClose,
  GateChecks,
} from "../../../wire/index";
import type { Scene } from "../../components/compositions/SketchScene/scene";
import type { SketchSettled } from "../../components/compositions/SessionThread/sketch-answer";
import type { Notice } from "../notices";
import type { RetroRowDrawn } from "../retro-row";
import type { PlanLandsIn } from "../../components/compositions/SessionThread/lands-in";

/** When a Session's work happened, each as an ISO instant. `until` is a slot given back. */
export type SessionTrail = {
  /** `SessionRecord.started_at`. */
  started: string;
  /** The slots it held, oldest first: a `slot` attachment's `since` and, once given back, `changed_at`. */
  bays?: readonly {
    slot: number;
    from: string;
    until?: string;
  }[];
  /** When each pull request was recorded, by number: a `pr` attachment's `since`. */
  opened?: Readonly<Record<number, string>>;
};

/** What a Session has attached, by kind. A Session holds several of each. */
export type SessionAttachment =
  /** A worktree slot the Session holds: leased on its first write, never at birth. */
  | {
      kind: "slot";
      slot: number;
    }
  | {
      kind: "branch";
      name: string;
      slot: number;
    }
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
  | {
      kind: "issue";
      number: string;
      title: string;
      address: string;
      state: "open" | "closed";
      /** The tracker's status as it said it, such as `Ready for QA`: `detail.status` on the wire. Absent where the tracker names none. */
      status?: string;
      pull_requests?: number[] /** The Job claimed it with `claim_issue`: work in hand, not one it filed or was told of. */;
      claimed?: true;
    }
  /**
   * A reported bug the Job was started on, by the report's id. Fleet keeps it while the Job stands; a Job that ends without
   * merging gives it back, and the report reads open again. Bridge finds a report's Job by this row.
   */
  | {
      kind: "report";
      id: string;
      title: string;
    }
  | {
      kind: "studio";
      id: string;
      title: string;
    }
  /**
   * A sketch the agent published into the Session, or one the person shared with it.
   * **The agent's is a scene, and a second one with the same title replaces the first under the same `id`.**
   * Its thread card is derived from the `show_sketch` call's tool row (`sketchCallOf`), never a row of its own.
   * `against` is the sketch it is read as a change to, held whole so a later replacement does not move it.
   * `asked` is one put to the person to approve, `drawn` the drawing he is shown, which `answerSketch` names back, and
   * `settled` how he answered: `detail.asked`, `detail.drawn` and `detail.settled` on the wire.
   */
  | {
      kind: "sketch";
      id: string;
      title: string;
      by: "agent" | "you";
      drawing: SessionSketch | Scene;
      against?: {
        title: string;
        drawing: SessionSketch | Scene;
      };
      asked?: true /** Taken back, by the agent or by him: it waits on nobody. */;
      withdrawn?: true;
      drawn?: string;
      settled?: SketchSettled /** Each drawing Fleet kept under the title, as a file of scene JSON, oldest first. The newest is `drawing`. */;
      revisions?: readonly string[];
    }
  /** `agentType` is the type the spawning call named, such as `rust-engineer`. Absent where the ledger holds none. */
  | {
      kind: "subagent";
      id: string;
      task: string;
      state: "running" | "done";
      report?: string;
      agentType?: string /** The Agent call that started it, which Fleet keys the Field's drone by. */;
      call?: string;
      /** The id of the plan group the agent named in its brief. Absent where it named none of the plan's. */
      group?: string;
    }
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
      /**
       * A document's newest revision put to him, `revision` its number as Fleet wrote it (`"2"`), which `answerDocument` names back, and
       * `settled` how he answered it: `detail.asked`, `detail.revision` and `detail.settled` on the wire. A new revision drops `settled`.
       */
      asked?: true;
      revision?: string;
      settled?: SketchSettled;
      /** When the newest revision was kept, as near as the record tells: its thread card stands at this time. */
      kept?: string;
      /** A document: the kept style set of each `mock` in its sketches, by content hash, keyed `<sketch title>#<node id>`: `detail.kept_styles` on the wire. */
      keptStyles?: Readonly<Record<string, string>>;
      /** A walk (form `window`) as its Job's tab draws it: the page's takes by name, and the steps, each pointing at a place on the page by name. **A mock's field**: nothing in Fleet carries it yet. */
      /** A walk (form `window`) as Fleet recorded it: the takes by name, the steps by their words and the brief, from the row's `detail`. **A walk is owed while its waiting item is**; this field says only what to draw. */
      window?: {
        brief?: string;
        takes: readonly string[];
        steps: readonly {
          say: string;
        }[];
      };
      walk?: {
        /** What the Job changed and what to look at, in its words. Absent on a walk shown without one. */
        brief?: string;
        takes?: readonly string[];
        steps: readonly {
          say: string;
          target: string;
        }[];
      };
    }
  /**
   * The Session this one was forked to, and the one it was forked from. `id` is the other Session's.
   * `moved` is present where the fork was a move to another repository: the id of the repository on the other side.
   */
  | {
      kind: "forked_to";
      id: string;
      moved?: string;
    }
  /**
   * `at` is the message the fork was taken from: the old conversation is shown up to and including it. Absent, the whole of it.
   * `moved` is the repository the Session was moved from and `branch` the branch it left there, comma-separated.
   */
  | {
      kind: "forked_from";
      id: string;
      at?: string;
      moved?: string;
      branch?: string;
    }
  /**
   * A Session this one's agent started when asked, and the Session whose agent started this one. `id` is the other Session's.
   * `group` is the plan group it was handed from, which draws it on that group's row. Every Session started from another is a linked Session (`linkedOf`)
   * and stands under it in Work, group or none.
   */
  | {
      kind: "started_to";
      id: string;
      group?: string;
    }
  | {
      kind: "started_by";
      id: string;
      group?: string;
    }
  /** A web address the person or the agent wrote in the thread. Loopback addresses and the repository's own issue and pull request links are not kept. */
  | {
      kind: "link";
      url: string;
      by: "you" | "agent";
    }
  /**
   * A note the person sent to the Job from the annotation layer: an `artifact` of form `note` on the wire, `noteArtifactOf` in
   * `classic-final:apps/desktop/src/shared/note-artifacts.ts`. `id` is the note's own. `picture` is an absolute PNG path, read as an artifact file is.
   */
  | {
      kind: "note";
      id: string;
      title: string;
      text: string;
      tag: string;
      component?: string;
      screen?: string;
      picture?: string;
      status: "open" | "done" | "returned";
      sent: string;
    };

/** A `window` is a page the Session showed in a window of Bridge's own; `id` is its address. */
export type SessionArtifactForm = "page" | "file" | "image" | "doc" | "document" | "window";

/**
 * A sketch as Dispatch and Studios hold one: boxes and the joins between them.
 * **The same shape the pad draws** (`SketchBox`, `SketchLine`), so the ledger
 * reads it with the preview Studios already draws.
 */
export type SessionSketch = {
  boxes: readonly {
    id: string;
    x: number;
    y: number;
    body: string;
  }[];
  lines: readonly {
    id: string;
    from: string;
    to: string;
  }[];
  /** Lines drawn by hand. Absent is none. */
  strokes?: readonly {
    id: string;
    points: readonly {
      x: number;
      y: number;
    }[];
  }[];
};

// ---- What a Session works to ---------------------------------------------------------------------------
// The plan, the workflow and the subagents on the field, drawn in the Session's own view on
// the Cockpit and on the Sessions page alike, from the record's plan, workflow, proposal and touches (`session-work.ts`). Progress is the agent's
// task states with what Fleet observed beside them (`seen`), and a status ring no longer stands for it.

export type WorkTaskState = "open" | "working" | "handed_in" | "done" | "failed";

/** One task the agent recorded. `seen` is whether Fleet observed edits for it. */
export type WorkTask = {
  id: string;
  text: string;
  state: WorkTaskState;
  seen?: boolean /** When the agent last changed this task's mark, RFC 3339. A mark the agent has not touched in a long while reads its age and dims. */;
  changedAt?: string /** Fleet set the state itself, and this is its proof ("commit 1a2b3c4 names it"). */;
  marked?: string /** In progress, and Fleet reads the work as elsewhere: why, and since when (RFC 3339). */;
  stale?: {
    why: string;
    since: string;
  };
};

/** Tasks one subagent takes, by the type of subagent. */
export type WorkGroup = {
  id: string;
  agent: string;
  title: string /** What the group gives him once it is done, one line in his words. The card reads it before the title. */;
  gives?: string /** The paths this group works in. */;
  paths: readonly string[] /** The issue this group works, as digits or a tracker's key. Its title and state are those of the Session's `issue` attachment of that number. */;
  issue?: string /** The pull request the group lands in. Absent on a Job that opens pull requests is PR 1. */;
  landsIn?: PlanLandsIn /** The group goes to a Job of its own, started with the group. Until it is handed, Bridge draws it as that Job, not started yet. Fleet sets it from the agent's `own_job`. */;
  ownJob?: true /** The fact Fleet sees that ends the group. Absent is none: the group runs on into the next as one stretch. */;
  gate?: Exclude<PlanGate, "none"> /** Fleet saw the gate's fact. Absent while it is open. */;
  gateClosed?: GateClose /** An open Checks gate: Fleet's Checks at the pushed commit. */;
  gateChecks?: GateChecks;
  tasks: readonly WorkTask[];
};

/** The scope as paths, the groups, and whether the person has approved it. */
/** A sketch the agent drew with the plan, in the format `show_sketch` takes. `id` is the sketch on the Session's ledger, which opens it. */
export type WorkSketch = {
  id: string;
  title: string;
  scene: Scene;
};

/** One line of what he sent, against the group or task it is about. */
export type SuggestLine = {
  verb: "Renamed" | "Removed" | "Added" | "Moved" | "Subagent" | "Note";
  where: string;
  text?: string;
  from?: string;
};

/** What the agent changed in the plan it recorded again: ids of what is new, what was renamed and what it was, and what is gone. */
export type PlanChanges = {
  added: readonly string[];
  renamed: readonly {
    id: string;
    from: string;
  }[];
  removed: readonly {
    group: string;
    text: string;
  }[];
};

export type WorkPlan = {
  /** The version the agent recorded, which approving and sending changes name. */ version?: number /** The plan in his words, a few lines of plain prose the card reads first. The groups are the technical detail under it. */;
  summary?: string;
  scope: readonly string[];
  groups: readonly WorkGroup[];
  approved: boolean /** He said Not yet: the plan stays recorded, folded to a chip in the thread, and nothing waits on him for it. */;
  setAside?: true /** What changed since the plan before this one. */;
  changes?: PlanChanges /** The sketch drawn with the plan, if the agent drew one. */;
  sketch?: WorkSketch /** The thread row that recorded it, which the approved plan settles after. */;
  after?: string /** The Job ended its work and is held on this group's open gate, with nothing of it running, since then. */;
  held?: {
    group: string;
    since: string;
  };
};

/** A drone of this Session over the area it is working in: the Session's own, or a subagent of `agent` type. */
export type WorkDrone = {
  id: string;
  agent?: string;
  task?: string;
  path: string;
  doing: "read" | "write" | "run" /** The file it is over, which the Field draws as a bar. */;
  file?: string /** When Fleet saw it touch that file. */;
  at?: string;
};

/** A file of the repository as the Field draws it, a bar on the plot of its directory. `act` is what is being done to it now: made, changed or taken away. */
export type WorkFile = {
  path: string;
  act?: "write" | "edit" | "remove";
};

/** One task on the agent's own list. The state is the plan task's, less the three a plan alone can say. */
export type AgentTask = {
  id: string;
  subject: string;
  state: "open" | "working" | "done";
};

export type SessionWork = {
  /** What the proposer read the request to be. Changeable at any time; a change applies from the next turn. */
  proposal?: {
    /** The id of the workflow chosen. */ workflow:
      | string
      | null /** The proposer's call is out: the Session starts on its answer. */;
    reading?: boolean /** What the proposer picked and the person has not changed: each is marked where it lives. */;
    picked?: readonly ("workflow" | "model" | "effort")[];
  };
  /** The workflow's steps and the index of the one the Session is at, where Fleet says. Absent is a custom Session, which has no rail; `at` absent draws the steps unlit. */
  workflow?: {
    name: string;
    steps: readonly string[];
    at?: number /** The phase of the step at `at`, as Fleet resolves it. */;
    phase?: StepPhase /** When each step began, in order, as an RFC 3339 time. A step the Job went back to appears again. */;
    visits?: readonly {
      step: number;
      since: string;
    }[] /** The steps the person skipped, by index; the Job passes over each. Absent where none is. */;
    skipped?: readonly number[];
  };
  /** Absent until the agent records its plan after planning with the person. */
  plan?: WorkPlan;
  /** The agent's own task list (its TaskCreate and TaskUpdate), in the order made. The spine draws `activity.todos` and not this. */
  tasks?: readonly AgentTask[];
  field?: readonly WorkDrone[];
  /** The files the Field draws as bars on its plots. */
  files?: readonly WorkFile[];
  /** The owner's setting turned workflows off: the picker, the steps and the proposer's mark for a workflow are not drawn. */
  railOff?: true;
  /** What the workflow ends in, which the Landing panel draws: the workflow's `lands`, as Fleet resolves it. Absent is a pull request. */
  landing?: WorkLanding;
};

/** What a workflow lands in: a pull request and its merge, a spike's findings doc, a mod and its switch, a document, or a report with no pull request. */
export type WorkLanding = "pull_request" | "spike" | "mod" | "document" | "report";

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

/**
 * `failed` is the required check red. `red` on the other two names a check that is red and not required:
 * on `pending` while the run goes on, on `passed` after the required checks passed.
 */
export type SessionChecks =
  | {
      state: "pending";
      red?: string;
    }
  | {
      state: "passed";
      red?: string;
    }
  | {
      state: "failed";
      failing: string;
    };

export type SessionAttachmentKind = SessionAttachment["kind"];

/** Who said a row of a Session's thread. **A Session that wrote to this one is named**, never folded into "agent". */
export type SessionVoice =
  | {
      kind: "you";
    }
  | {
      kind: "agent";
    }
  | {
      kind: "session";
      id: string;
      title: string;
    };

/** What a followed Session did that its parent is told of. */
export type LinkedEvent = "needs" | "done" | "merged" | "started";

type SessionRowKinds =
  | {
      id: string;
      at: string;
      kind: "message";
      from: SessionVoice;
      text: string;
      /** What the person sent with it. */
      files?: readonly SentFile[];
      sketches?: readonly {
        id: string;
        title: string;
      }[];
      /** Sessions tagged with `@`, so the agent knows to talk to them. */
      tags?: readonly SessionTag[];
      /** The Fleet notice this row was sent for, which a person can turn off. */
      notice?: Notice;
      /** A message of the person's that Fleet has not yet taken (`sending`) or did not (`failed`). Absent once it is Fleet's row. */
      delivery?: "sending" | "failed";
      /** What the row is drawn under while it changes from the message Bridge held to the row Fleet sent back, so it does not draw again. */
      stableKey?: string;
      /** The person's words, held until the call the agent's turn waits on returns. */
      queued?: true;
    }
  /**
   * Words an old thread sent on to a Session or a Job. Only an ended Job's thread has these: Fleet keeps the row
   * beside the ones the agent wrote, in the thread where it was sent.
   */
  | {
      id: string;
      at: string;
      kind: "sent";
      text: string;
      /** The Session the words went to, and the repository it is of as the picker names it. */
      target: ThoughtTarget & {
        repository: string;
      };
      files?: readonly SentFile[];
      sketches?: readonly {
        id: string;
        title: string;
      }[];
      tags?: readonly SessionTag[];
      /** What the row is drawn under in place of the words Bridge held while they went, so it does not draw again. */
      stableKey?: string;
    }
  /** A tool call, mono. `fromPhone` is an ask the person answered on their phone; `subagent` the one an `Agent` call the turn waits on started. */
  | {
      id: string;
      at: string;
      kind: "tool";
      text: string;
      fromPhone?: true;
      subagent?: ToolSubagent;
    }
  /** A command the person ran in the terminal, as typed. */
  | {
      id: string;
      at: string;
      kind: "command";
      text: string;
    }
  /** The summary the CLI wrote where it compacted the conversation: not the person's words. */
  | {
      id: string;
      at: string;
      kind: "compaction";
      text: string;
    }
  /** How long the agent reasoned before what it did next, and the CLI's summary of what it reasoned. */
  | {
      id: string;
      at: string;
      kind: "reasoning";
      seconds: number;
      text: string;
      /** Still reasoning: the words so far. */
      live?: true;
    }
  /** The agent disputed Fleet's notice `notice`: its reason, how it was spotted, and the review Job where one was started. */
  | {
      id: string;
      at: string;
      kind: "disputed";
      notice: string;
      why: string;
      spotted: "called" | "read";
      review?: string;
    }
  /** The agent reached for a browser outside Bridge: `tool` as the person reads it, and the address it was on. Drawn by the Job browser's reminder, not the thread's own rows. */
  | {
      id: string;
      at: string;
      kind: "browser_reminder";
      tool: string;
      address?: string;
    }
  /** The agent's first write: the slot leased and the branch cut, drawn in the thread where it happened. */
  | {
      id: string;
      at: string;
      kind: "lease";
      slot: number;
      branch: string;
    }
  /** The slot given back, drawn in the thread where it happened. */
  | {
      id: string;
      at: string;
      kind: "release";
      slot: number;
      branch: string;
    }
  /** A value the person changed through a panel, which the agent was told of: what it was and what it is now. */
  | {
      id: string;
      at: string;
      kind: "change";
      what: string;
      from: string;
      to: string;
    }
  /** The changes and notes he sent on the plan, one message with a line for each. */
  | {
      id: string;
      at: string;
      kind: "suggest";
      lines: readonly SuggestLine[];
    }
  /** An `armada check` the agent ran in its slot: the Check's key, its run on the Checks page, where it stands, and where Fleet read it failed. */
  | {
      id: string;
      at: string;
      kind: "check";
      name: string;
      run: number;
      state: "running" | "passed" | "failed" | "timed_out";
      target?: string;
      blame?: CheckBlame;
    }
  /** A command the person ran from the thread or the band, which the Session was told: its exit (`code` is null where it was stopped) and the last lines it printed, as plain text. */
  | {
      id: string;
      at: string;
      kind: "ran";
      command: string;
      code: number | null;
      tail: string /** Still running, since `since`. **A mock's field**: Fleet tells the Session of a run only when it ends. */;
      running?: true;
      since?: string;
    }
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
  | {
      id: string;
      at: string;
      kind: "saved";
      name: string;
      collection: string;
      collectionId: string;
      request: string;
    }
  /** The Session showed a page in a window: where in the thread it did, and the address to open again. `ended` is when the walk ended; absent while it is open. */
  | {
      id: string;
      at: string;
      kind: "window";
      title: string;
      url: string;
      ended?: string /** How many takes of the change the page shows. **A mock's field**: nothing in Fleet carries it yet. */;
      takes?: number;
    }
  /** A draft the agent put to the person with `propose_draft`, whole as Fleet wrote it; `settled` once he answered it. */
  | {
      id: string;
      at: string;
      kind: "draft";
      title: string;
      text: string;
      before?: string;
      settled?: DraftSettled /** The workflow change a proposed workflow is. */;
      change?: number /** **A mock's field**: a workflow a Job proposed, whose card names it and where it would be kept. */;
      workflow?: {
        name: string;
        scope: "kit" | "repository";
      } /** **A mock's field**: a draft that is a rule's own act, whose card is the rule and says where it goes and to whom. */;
      rule?: {
        where: string;
        to: string;
      };
    }
  /** This Session's agent started a Session, or forked itself into one: the new Session, to open from the thread. */
  | {
      id: string;
      at: string;
      kind: "started";
      session: {
        id: string;
        title: string;
      };
      fork: boolean;
    }
  /**
   * A Session that waits on this one: it was dispatched to, its start condition was changed by this Session's agent or by the person,
   * or its condition was met and it started. `by` is who changed the condition. **A mock's row**: nothing in Fleet carries it yet.
   */
  | {
      id: string;
      at: string;
      kind: "waiter";
      session: {
        id: string;
        title: string;
      } /** Written in the waiting Session's own thread: `session` is the one it waits on. */;
      waiting?: true;
      event: "waits" | "changed" | "started";
      says?: string;
      from?: string;
      to?: string;
      by?: "you" | "agent";
    }
  /** A Session this one follows needs the person, finished, or merged its pull request. The agent was told in the same moment. */
  | {
      id: string;
      at: string;
      kind: "linked";
      session: {
        id: string;
        title: string;
      };
      event: LinkedEvent;
    }
  /** The Operator started this Job, sent it words or answered for the person, in those exact words. */
  | {
      id: string;
      at: string;
      kind: "operator";
      text: string;
    }
  /** A retro being written for the Session, then the one that was: the latest frame of one press's row. */
  | RetroRowDrawn;

/** Every row's `at` is the RFC 3339 instant it happened. A mock's may be words, and the stamp then draws them as they came. */
export type SessionRow = SessionRowKinds;

/** One item of the agent's own TodoWrite list. The agent's report of itself; `SessionActivity.now` is what Fleet detected. */
export type SessionTodo = {
  text: string;
  state:
    | "done"
    | "doing"
    | "todo" /** The id of the plan task Fleet matched this item to. Its plan row already shows it. */;
  task?: string;
};

/**
 * What a Session is doing, from Fleet's PreToolUse gate: `now` is the last tool call as a short
 * action, and `todos` the agent's list where it keeps one. Todos are detail on top of `now`, never a
 * plan the Session is held to.
 */
export type SessionActivity = {
  now?: string;
  todos?: readonly SessionTodo[];
  /** Each subagent running now, by its Agent call: `label` is what it was sent to do, `now` its own last tool call as a short action. */
  subagents?: readonly {
    id: string;
    label: string;
    now?: string;
  }[];
  /** The last sentence the agent wrote this turn, for a surface that draws what it said beside what it does. */
  said?: string;
  /** The newest lines each running background command wrote, by the id of the call that started it. */
  tails?: readonly {
    call: string;
    lines: readonly string[];
  }[];
};

/** A control pressed on a Session whose answer is still coming: Pause or Resume, Auto land, and Approve on the plan. */
export type PressedKey = "paused" | "autoLand" | "plan";

/** A turn running, or none. A message from another Session starts one, so `working` has no author. */
export type SessionTurn =
  | {
      state: "idle";
    }
  | {
      state: "working";
      wokenBy?: {
        id: string;
        title: string;
      };
    };

// ---- A Session that waits on another -------------------------------------------------------------------------
// Draft, mock-only: nothing in Fleet or the protocol carries it. A Session dispatched after another holds no slot and
// runs nothing until its condition is met; the Session it waits on is told, and may change the condition.

/** What starts a waiting Session: the other Session ends, a part of its plan is done, its pull request is pushed, or lands. */
export type StartWhen = "ends" | "plan" | "pushed" | "lands";

export type SessionWait = {
  /** The Session it waits on. */
  on: {
    id: string;
    title: string;
  };
  when: StartWhen;
  /** With `plan`: the group or task of the other Session's plan, by id. */
  part?: {
    id: string;
    title: string;
  };
  /** With `pushed` and `lands`: the pull request, once the other Session has named it. */
  pull?: {
    number: number;
    branch: string;
  };
  /** Where it starts from: main, or the branch of the other Session's pull request. */
  base: StartBase;
  /** What it needs of the Session it waits on: the rest of the clause the brief names the wait in. Empty where the brief names none. */
  needs: string;
  /** The words it was dispatched with, which it starts on. */
  brief: string;
  /** What happens once the condition is met: absent or true opens it as a draft (held); false is Approved, which starts it at once. */
  held?: boolean;
  /** The workflow it starts under, by id; absent is the proposer's. */
  workflow?: string;
  /** The model and effort it starts on; absent is the default. */
  model?: string;
  effort?: string;
  /** Who last set the condition. */
  by: "you" | "agent";
};

export type StartBase = "main" | "branch";

/** What the agent's init line showed it lacks: `mod` did not load, `door` (Fleet's MCP door) did not connect, `task_tools` were not served. */
export type ToolsMissing = {
  harness: string;
  update: string;
  harness_version: string;
  missing: Array<"mod" | "door" | "task_tools">;
};

// ---- What Fleet saw a Job do, and what it did for it ---------------------------------------------------------
// A Job that stopped says why and what Fleet is doing about it, and every act Fleet took on the Job's behalf stands as one more worker beside the agent.

/** Why a Job stopped: it ended its turn with nothing running and nothing asked, with tasks open or not, its process ended, or Fleet was down. */
export type StoppedReason = "turn_ended" | "tasks_open" | "process_gone" | "fleet_down";

/** A Job that is not running and was not parked. `at` is when Fleet saw it; `restart` is where Fleet stands on bringing it back. */
export type Stopped = {
  reason: StoppedReason;
  at: string;
  /** With `process_gone`: the last thing the process said. */
  said?: string;
  /** With `tasks_open`: the tasks still open on its list. */
  tasks?: readonly string[];
  restart: "none" | "queued" | "running" | "failed";
  /** Where a repair Job stands, once a restart failed. `held`: Fleet is unhealthy, so it started none. */
  repair?: "running" | "failed" | "held";
  /** Fleet has run out of things to try: only he moves it now. No item: the mark and the call reach him, as stuck does. */
  gaveUp?: true;
  /** With `fleet_down`: the stretch Fleet was not running. */
  down?: {
    from: string;
    to: string;
  };
};

/** What Fleet does on a Job's behalf, in the kinds Fleet acts in. */
export type FleetActKind =
  | "told"
  | "held"
  | "process"
  | "step"
  | "slot"
  | "judge"
  | "restart"
  | "nudge";

/** One act of Fleet's on a Job. `under_way` stands in Now, `coming` in Open, `over` in Done. `why` is the reason; `until` is when a coming act happens. */
export type FleetAct = {
  id: string;
  at: string;
  state: "under_way" | "coming" | "over";
  kind: FleetActKind;
  said: string;
  why?: string;
  until?: string;
  /** The workflow step the Job was on when Fleet did it. Absent where it was on none. */
  step?: string;
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
  /**
   * The Job's agent process never started, even on Fleet's second try (`hosted.start_failed`). `cause` is why, in plain words; `log` is the agent's own
   * last error lines. The Job needs him: Start again or a message from him is the retry. Absent once it has started.
   */
  startFailed?: {
    cause: string;
    log: readonly string[];
  };
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
  /** The agent came up without some of its Armada tools. Absent when nothing is missing. */
  toolsMissing?: ToolsMissing;
  /** The commands a terminal session lists, where its mod has said. A hosted one uses the draft's. */
  commands?: readonly SessionCommand[];
  /** Set where the rows are not all held, so a session whose thread was never opened is not taken for a blank one. */
  blank?: boolean;
  /** Held until the plan's limit resets, as ISO: Fleet wakes it then. Absent unless it is held. */
  limitedUntil?: string;
  /** How full its context window is, as last reported. Absent until it has reported one. */
  context?: {
    tokens: number;
    window: number;
  };
  /** What fills that window, as the harness reports it. Absent where it has not. */
  contextBreakdown?: ContextBreakdown;
  /** What its agent knows and was told, counted, and what the latest turn used. Absent until the Session reports one. */
  loadout?: LoadoutSummary;
  /** A Mod Job: started from Mod, for modding Armada itself. */
  mod?: true;
  /** A Mod Job that builds a Custom tab: the Job whose panes hold the tab, and the tab's id. It has no row of its own; the tab and its card are where it lives. */
  modFor?: {
    job: string;
    tab: string;
  };
  /** A Mod Job that builds an instrument for a bay of the console: the bay and its card are where it lives. */
  modOfBay?: true;
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
  versions?: readonly {
    at: string;
    attachment: SessionAttachment;
    from?: {
      message?: string;
      task?: string;
    };
  }[];
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
   * The questions the agent asked that nobody answered in time and are still open.
   * `id` is the ask's row. Not `asked`: that is a live call the agent is held on.
   */
  lapsed?: {
    id: string;
    questions: readonly SessionQuestion[];
  };
  /** What it is doing now and the agent's todo list. Absent is nothing to say. */
  activity?: SessionActivity;
  /** The running account of a turn in his words, answering the message row `for`: what is done, what it is on now and what is next. No file or tool names. */
  account?: {
    for: string;
    done: readonly string[];
    now?: string;
    next?: string /** The id of the approved plan's group the now line belongs to; named in that line and marked on its card. */;
    group?: string;
    at: string;
  };
  /** The page or sketch the agent is building, drawn live beside the account. */
  live?: {
    url: string;
    title: string;
  };
  /** What it has built: the phase, a piece for each task, the road and the subagents at work, as Fleet derived them. Absent where Fleet observed nothing. */
  build?: SessionBuild;
  /** What the Session waits on the person for: the agent's list with Fleet's own items merged in. Absent or empty is nothing. */
  waitingFor?: readonly WaitingItem[];
  /** What he closed from Needs you, kept to be found again. `ended` is an item whose thing is over (its call finished, its walk withdrawn), which no Reopen brings back. */
  closed?: readonly {
    item: WaitingItem;
    at: string;
    ended?: boolean;
  }[];
  /** The follow-ups the agent kept for itself and has not settled, oldest first. `SessionRecord.later`; `id` is what a Drop names. */
  later?: readonly {
    id: string;
    text: string;
    at: string /** The person marked it Deferred: Fleet was right to keep it, and its thumbs are gone. */;
    confirmed?: boolean;
  }[];
  /** Everything that is not him that keeps it open, in the order it reads. `SessionRecord.held_open_by`. */
  heldOpenBy?: readonly HeldOpenBy[];
  /** Why it is not running, where it stopped without being parked. Absent for a Job that runs, rests or was paused. */
  stopped?: Stopped;
  /** The repair Fleet is running on it, in any state: Fleet's own words for what broke, and since when as ISO. Absent while none runs. */
  repairing?: {
    said: string;
    since: string;
  };
  /** When the running turn began, as ISO. */
  turnSince?: string;
  /** The last time Fleet heard anything from the running turn, as ISO. A turn silent for long is `stuck`. */
  lastHeardAt?: string;
  /** The tool call the turn is in, and since when: what the agent row names while a turn runs. */
  openCall?: {
    text: string;
    since: string;
  };
  /** Everything Fleet does or will do for this Job, newest last. */
  fleetActs?: readonly FleetAct[];
  /** The agent says it has nothing left to work on, since when. `SessionRecord.done_since`. A message from the person clears it. */
  done?: {
    since: string;
    summary?: string;
  };
  /** What the Job has done, in a sentence or two. `SessionRecord.so_far`. The Calls HUD draws it where it is present (`ConsoleCalls.tsx`). */
  soFar?: string;
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
  start?: {
    asksDelivery: boolean;
    picked?: DeliveryPicked /** The first message the agent will read, held in the composer for him to edit. Absent until one is kept for the Job. */;
    message?: string /** Start was pressed while the proposer read: it starts when the reading ends. */;
    queued?: boolean;
  };
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
  started?: {
    workflow?: string;
    delivery?: DeliveryChoice;
    model?: string;
    effort?: string;
    mode?: SessionMode /** The Job it was started from. */;
    by?: {
      job: string;
      title: string;
    };
  };
  /** The plan, workflow and subagents the view draws round the thread. Absent draws the thread and its ledger as before. */
  work?: SessionWork;
  /** What a Judge read of the Job's artifacts, one per artifact, the latest replacing the one before (`SessionRecord.judge_readings`). Absent is none asked. */
  judgeReadings?: readonly JudgeReading[];
  /** Dispatched after another Session and not started: it holds no slot and runs nothing until the condition is met. Absent once it has started. */
  waits?: SessionWait;
};

/**
 * Something a message tags with `@`, or an issue or pull request with `#`, so the agent knows what
 * is meant: another Session, a pull request, a branch or an issue.
 */
export type SessionTag = {
  kind: "session" | "pull_request" | "branch" | "issue" | "report" | "terminal";
  id: string;
  title: string;
  /** The forge's address of an issue or a pull request. */
  url?: string;
};

/** A skill or command `/` offers, as a terminal session lists them. */
export type SessionCommand = {
  name: string;
  says: string;
};

/** What a person may answer a permission a Session is held on. */
export type SessionAnswer =
  | "allow_once"
  | "allow_and_remember"
  | "refuse"
  | "always_allow"
  | "always_block"
  | "allow_for_session"
  | "allow_like_this_for_session";

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
  /** Whether the call reads that path, writes to it, or runs something the guard cannot class. */
  reachesTo?: "read" | "write" | "run";
  /** The Bash call's program and script, which "Allow calls like this for this job" allows a width of. */
  kind?: CallKind;
};

/** One question the agent put to the person. */
export type SessionQuestion = {
  question: string;
  header: string;
  multi_select: boolean;
  options: readonly {
    label: string;
    description: string;
    preview?: string;
  }[];
};

/** Whether a Session has leased anything yet. A blank one has no slot and no branch. */
export function isBlank(session: Session): boolean {
  return session.blank ?? (session.attachments.length === 0 && session.rows.length === 0);
}

/** Whether the Job lands its work in pull requests: its delivery opens or lands one, or it already has one. Keep local with none opened does not. */
export const opensPullRequests = (session: Session): boolean =>
  session.started?.delivery === "open_pr" ||
  session.started?.delivery === "auto_land" ||
  attachmentsOf(session, "pull_request").length > 0;

export function attachmentsOf<K extends SessionAttachmentKind>(
  session: Session,
  kind: K,
): Extract<
  SessionAttachment,
  {
    kind: K;
  }
>[] {
  return session.attachments.filter(
    (
      one,
    ): one is Extract<
      SessionAttachment,
      {
        kind: K;
      }
    > => one.kind === kind,
  );
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
export const planAsked = (session: Session): boolean =>
  owesOf(session).some((one) => one.source === "plan");

/**
 * What the Session is doing: Fleet's reading where it gave one, else what the turn alone says. A Fleet
 * that predates `standing` never says `replied` or `stuck`, and a Session it called done still is.
 */
export const standingOf = (session: Session): SessionStanding =>
  session.standing ??
  (session.turn.state === "working" ? "working" : session.done !== undefined ? "done" : "idle");
