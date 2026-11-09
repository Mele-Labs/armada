// What the Operator panel draws, typed here because no wire carries it yet. Each shape is a proposal for the protocol, listed in the
// header of the walk `operatorFull` (`mock/walks/operator.ts`). The mock scenario seeds it; the panel's own presses move it locally.
// `line` is the list the panel calls Waiting to start.

/** What the Operator did with an item, the Log's third column. */
export type OperatorDid =
  | "answered"
  | "raised"
  | "delegated"
  | "existing"
  | "merged"
  | "in_touch"
  | "held"
  | "started"
  | "filed"
  | "passed_on"
  | "released"
  | "returned"
  | "resumed"
  | "moved"
  | "overruled";

/** The four roles a person turns. Evaluate is the Operator switch itself; Raise has no control. */
export type OperatorRole = "dispatch" | "delegate" | "answer" | "file";

/** How far the Operator may go with what a role covers, as a browser's tool permissions are set. */
export type OperatorPosition = "read_only" | "suggest" | "decide";

/** A Job a row names, drawn as the Job card that opens it. `title` is what it was called, for a Job the window no longer holds. */
export type OperatorJob = {
  session: string;
  title: string;
};

/** What a held item waits for. Fleet refuses a hold that names neither a Job nor a meter; `you` is a row the person moved there. */
export type OperatorWait =
  | {
      kind: "job";
      job: OperatorJob;
      event: "pushes" | "ends";
      /** The plan group the push is of. */
      group?: string;
    }
  | {
      kind: "meter";
      meter: "five_hour" | "weekly";
      /** The percentage the meter has to fall under. */
      under: number;
    }
  | {
      kind: "you";
    };

/** What started an item: your dispatch or correction, a Job, a bug report and who filed it, Fleet, or a row waiting to start. */
export type OperatorSource =
  | {
      kind: "you";
      did: "dispatched" | "corrected";
    }
  | {
      kind: "job";
      job: OperatorJob;
    }
  | {
      kind: "report";
      by: OperatorJob | "you";
    }
  | {
      kind: "fleet";
      /** What Fleet did, such as "Fleet restarted". */
      did: string;
    }
  | {
      kind: "waiting";
    };

/** How a ruling names another Job: the one the words went to, the one merged in, or the one kept in touch with. */
export type OperatorNamed = OperatorJob & {
  as: "existing" | "merged" | "in_touch" | "nudged";
};

/** One act, newest first. */
export type OperatorLogRow = {
  id: string;
  /** ISO time it was ruled. */
  at: string;
  from: OperatorSource;
  /** The item itself: a question, a request, a report. */
  item: string;
  did: OperatorDid;
  why: string;
  /** Every Job the ruling names. */
  names?: readonly OperatorNamed[];
  /** The row a hold put in Waiting to start, which Start now on this row starts. */
  line?: string;
  /** The other outcomes Overrule offers. Absent draws no Overrule. */
  ways?: readonly string[];
  /** The outcome the person overruled it to, a correction the next call reads. */
  changed?: string;
  /** The Manifest it arose in, which the title row's picker narrows the Log to. */
  repository?: string;
  /** Who did it. Absent, the Operator. */
  by?: "you" | "operator";
  /** The outcome's own words after its kind: the answer given, `to <Job>`, `with <Job>`. */
  what?: string;
  /** What the item was, which names it in the row's sentence. */
  about?: "question" | "stall" | "report" | "start" | "note";
};

/** The person switching the Operator on or off, a line across the Log. */
export type OperatorSwitch = {
  on: boolean;
  /** ISO time. */
  at: string;
};

/** One item that has not started, in the Operator's order. */
export type OperatorLineRow = {
  id: string;
  title: string;
  waits: OperatorWait;
  /** Why it waits on that, in the Operator's words. */
  why?: string;
  /** The Job it was dispatched as, whose header says it is held. */
  session?: string;
  /** The Manifest it would start in, which the title row's picker narrows the list to. */
  repository?: string;
};

/**
 * What a ruling on who does the work left on a live Job's row, naming the other Job. Fleet refuses one that names no shared paths
 * or plan group. A Job another was merged into names the Job it took in; two Jobs kept in touch each name the other.
 */
export type OperatorTie = {
  session: string;
  kind: "merged" | "in_touch";
  /** The other Job's title: the one taken in, or the one kept in touch with. */
  other: string;
  /** The paths the two share. */
  paths: string;
  /** The plan group of this Job that waits on the other's push, where one must build first. */
  waits?: string;
};

/** A standing order, only ever in one of the forms Fleet enforces. */
export type StandingOrder = {
  id: string;
} & (
  | {
      kind: "role_off";
      role: OperatorRole;
      repository: string;
    }
  | {
      kind: "hold_paths";
      paths: string;
      until: {
        job: string;
        event: "pushes" | "ends";
      };
    }
  | {
      kind: "always_raise";
      what: string;
    }
);

/** What the Operator has made of a Job's question: still reading it, or raised with its read and pick. */
export type OperatorRead =
  | {
      session: string;
      state: "reading";
    }
  | {
      session: string;
      state: "raised";
      read: string;
      pick: string;
    };

/** One message in the box's conversation. `order` is a standing order the words came to, which stands only once he presses Keep. */
export type OperatorMessage = {
  id: string;
  from: "you" | "operator";
  text: string;
  order?: StandingOrder;
  /** What he did with `order`. */
  settled?: "kept" | "left";
  /** The Waiting on you row this marks the place it came up for discussion, in place of words. */
  card?: string;
  /** The card marks where that row was settled, rather than where it came up. */
  ends?: boolean;
  /** ISO time he sent it, for his own message's band. */
  at?: string;
  /** A Job this started, which the message opens. */
  opens?: OperatorJob;
};

/** What the Operator would do about an item once the two of them have talked: the press that names it, what the row then says, and its line until then. */
export type OperatorSend = {
  press: string;
  result: string;
  read: string;
};

/** The mock's script for one item's conversation: what it says as the item opens, and to words that hold `asked`. */
export type OperatorItemTalk = {
  first: string;
  replies: readonly {
    asked: string;
    said: string;
    send?: OperatorSend;
  }[];
};

/** One thing the Operator has in hand, in the order it gets to them. The first is the one it is reading. */
export type OperatorTurn = {
  id: string;
  text: string;
  /** His message this is, in the conversation. */
  message?: string;
  /** The repository it arose in, so the picker narrows it. */
  repository?: string;
  /** Under its title while nothing else is true of it. */
  about?: string;
  /** Where it is a Job waiting to start: the repository's line it stands in, and what holds it other than a free slot, read after "Starts when". */
  start?: {
    repository: string;
    waits?: string;
  };
  /** The Waiting on you row this stands for: one it brings up in Discussion, or a press it has still to carry out. */
  raise?: string;
  /** Bring `raise` up in Discussion when it gets here. */
  discuss?: boolean;
  /** What `raise` says once it is carried out. */
  result?: string;
  /** Which way he last moved it. */
  moved?: "up" | "down";
  /** What the Operator did to put it first when he asked in words. */
  put?: string;
  /** What the Operator says came of it once it has done the row, if anything did. */
  finish?: string;
  /** What came of it. It stays below what is still open, saying so. */
  done?: string;
  /** Who settled it, as its card says: Operator or You. */
  by?: string;
  /** ISO time it was settled. */
  at?: string;
  /** Why, in the words kept with the ruling. */
  because?: string;
  /** The Job it was about, which its card opens. */
  job?: OperatorJob;
  /** The tag before its title, where the kind of thing is known outright, as Fleet's board says. */
  tag?: string;
};

/** The mock's script for the box: what a fresh call answers to words that hold `asked`, the Line row it starts and the order it proposes, if any. */
export type OperatorTalk = {
  asked: string;
  said: string;
  starts?: string;
  order?: StandingOrder;
  /** The row of Working through these words put next, and what its row then says. */
  puts?: {
    turn: string;
    line: string;
  };
};

/**
 * Something raised, or what a role at Suggest would do, tied to no Job: a report, an issue, a review request, a red main or a Later
 * item. `ways` are its presses, the first the Operator's pick.
 */
export type OperatorRaise = {
  id: string;
  /** Whether a role at Suggest would do it, which `Do it` does, or it was raised. */
  suggests: boolean;
  repository: string;
  /** The Manifest it arose in, which the title row's picker narrows the list to. */
  manifest?: string;
  item: string;
  /** Why it came to him, in the Operator's words. */
  read: string;
  ways: readonly string[];
  /** The way the Operator would pick, or the Job marked as its own pick: Go with it presses it. */
  recommends?: string;
  /** The branch its Job's work is on, which a review reads. */
  branch?: string;
  /** What the Log says the pick did. */
  does: OperatorDid;
  /** What kind of thing it is, the tag before its title. */
  tag?: string;
  /** What the row says once a way is pressed. */
  results?: Readonly<Record<string, string>>;
  /** The ways the Operator carries out in its turn: pressed, they go in its queue. The rest happen at once. */
  queues?: readonly string[];
  /** Waiting in the Operator's queue to come up in Discussion. */
  toDiscuss?: boolean;
  /** What came of it. The row stays where it is, saying so. */
  settled?: string;
  /** Who settled it, as its card says, and the ISO time. */
  settledBy?: string;
  settledAt?: string;
  /** The Job it is about, which Open Job opens. */
  job?: OperatorJob;
  /** The mock's script for working it through in Talk. */
  talk?: OperatorItemTalk;
};

/** A repository whose roles differ from the machine's, a setting rather than an order. A role it does not name follows the machine. */
export type OperatorRepository = {
  name: string;
  positions: Partial<Readonly<Record<OperatorRole, OperatorPosition>>>;
};

export type OperatorSeed = {
  /** The Operator switch: off sends everything to the person as today. */
  on: boolean;
  positions: Readonly<Record<OperatorRole, OperatorPosition>>;
  /** The share of the week's plan, in percent, past which it pauses and everything passes through. */
  pause: number;
  /** While it is paused: where the week stands and the share it paused at. */
  paused?: {
    week: number;
    at: number;
  };
  log: readonly OperatorLogRow[];
  /** Each time the person switched it on or off, newest first. */
  switched?: readonly OperatorSwitch[];
  line: readonly OperatorLineRow[];
  orders: readonly StandingOrder[];
  reads: readonly OperatorRead[];
  ties: readonly OperatorTie[];
  talk: readonly OperatorTalk[];
  raises?: readonly OperatorRaise[];
  repositories?: readonly OperatorRepository[];
  /** Every repository a difference could be added for. */
  served?: readonly string[];
  /** The model its calls run on, the same setting as the system models' row. */
  model?: string;
  models?: readonly string[];
  /** Where a raise tied to no Job pulses: on the title row's button alone, or on a row of the Jobs list too. */
  raisesPulse?: "button" | "row";
  /** What it has in hand. Empty or missing, a message is answered at once. */
  turns?: readonly OperatorTurn[];
  /** The whole-panel take this scenario draws (`operator/takes/`). Absent draws the panel as built, or the one `?take=` names. */
  take?: string;
  /** Discussion takes an item at once, however busy the Operator is: Fleet's Operator holds no queue for talk. */
  discussesAtOnce?: boolean;
  /** Takes what moves the Operator on to its next turn, where time passes in the scenario. */
  moves?: (next: () => void) => void;
};

/** The line under a row of the Operator's Queue: what came of it, what the Operator did to put it first, or what it waits on. */
export function turnLine(turns: readonly OperatorTurn[], at: number): string | undefined {
  const row = turns[at]!;
  if (row.done !== undefined) return row.done;
  if (row.put !== undefined) return row.put;
  if (row.start === undefined) return row.about;
  const waits = `Starts when ${row.start.waits ?? "a slot is free"}`;
  return row.about === undefined ? waits : `${row.about}. ${waits}`;
}

/** An outcome as its kind and what, from "Kind: what". An outcome that let the item go is drawn quieter. */
export function outcomeOf(text: string): {
  label: string;
  what?: string;
  letGo: boolean;
} {
  const at = text.indexOf(": ");
  const label = at === -1 ? text : text.slice(0, at);
  return {
    label,
    ...(at === -1
      ? {}
      : {
          what: text.slice(at + 2),
        }),
    letGo: label === "Ignored" || label === "Not filed",
  };
}

/** The tag before a row's title in the Operator's Queue: what kind of thing it is. */
export function turnTag(row: OperatorTurn): string {
  if (row.tag !== undefined) return row.tag;
  if (row.message !== undefined) return "Your message";
  if (row.discuss === true) return "To discuss";
  if (row.result !== undefined) return "To do";
  if (row.start !== undefined) return "New Job";
  return "Question";
}

export const ROLES: readonly {
  role: OperatorRole;
  name: string;
  does: string;
}[] = [
  {
    role: "dispatch",
    name: "Dispatch",
    does: "Starts or holds what you dispatch",
  },
  {
    role: "delegate",
    name: "Delegate",
    does: "Sends words to a live Job",
  },
  {
    role: "answer",
    name: "Answer",
    does: "Questions from Jobs on groups you will not see",
  },
  {
    role: "file",
    name: "File",
    does: "New issues, after a duplicate check",
  },
];

export const POSITION_SAID: Readonly<Record<OperatorPosition, string>> = {
  read_only: "Read-only",
  suggest: "Suggest",
  decide: "Decide",
};

/** What each level does, under its name in the menu. */
export const POSITION_DOES: Readonly<Record<OperatorPosition, string>> = {
  read_only: "Sees it and does nothing",
  suggest: "Puts its pick in Needs you",
  decide: "Does it on its own",
};

export const DID_SAID: Readonly<Record<OperatorDid, string>> = {
  answered: "Answered",
  raised: "Raised",
  delegated: "Delegated",
  existing: "Existing Job",
  merged: "Merged",
  in_touch: "Kept in touch",
  held: "Held",
  started: "Started",
  filed: "Filed",
  passed_on: "Sent to you",
  released: "Released",
  returned: "Returned",
  resumed: "Resumed",
  moved: "Moved",
  overruled: "Overruled",
};

const METER_SAID = {
  five_hour: "5 hour window",
  weekly: "Weekly window",
} as const;

/** What a row waiting to start waits for, in a held Job's head and its row's tooltip. */
export function waitSaid(wait: OperatorWait): string {
  switch (wait.kind) {
    case "job":
      return wait.event === "ends"
        ? `${wait.job.title} ends`
        : wait.group === undefined
          ? `${wait.job.title} pushes`
          : `${wait.job.title} pushes its ${wait.group} group`;
    case "meter":
      return `${METER_SAID[wait.meter]} under ${wait.under}%`;
    case "you":
      return "You moved it here";
  }
}

/** What a row in Waiting to start says it waits on, the Job it waits on drawn beside it as its card. */
export function waitLine(wait: OperatorWait): string {
  switch (wait.kind) {
    case "job":
      return wait.event === "ends"
        ? "Starts when this Job ends"
        : wait.group === undefined
          ? "Starts when this Job pushes"
          : `Starts when this Job pushes its ${wait.group} group`;
    case "meter":
      return `Starts when the ${METER_SAID[wait.meter].toLowerCase()} is under ${wait.under}%`;
    case "you":
      return "Waits on you. You moved it here";
  }
}

/** What started an item, in words: the Log row's name, and its first column where no Job card stands for it. */
export function sourceSaid(from: OperatorSource): string {
  switch (from.kind) {
    case "you":
      return from.did === "dispatched" ? "You dispatched" : "You";
    case "job":
      return from.job.title;
    case "report":
      return from.by === "you" ? "Bug report from you" : `Bug report from ${from.by.title}`;
    case "fleet":
      return from.did;
    case "waiting":
      return "Waiting to start";
  }
}

/** The Overrule menu's outcomes for a ruling on who does the work, less the one it made. `other` is the Job the ruling named. */
export function outcomesBut(did: OperatorDid, other: string): readonly string[] {
  return [
    did === "started" ? undefined : "New Job",
    did === "existing" ? undefined : `Existing Job: send it to ${other}`,
    did === "merged" ? undefined : `Merge with ${other}`,
    did === "in_touch" ? undefined : `Keep in touch with ${other}`,
    "Raise to me",
  ].filter((one): one is string => one !== undefined);
}

/** What a Job row's tie glyph says, naming the other Job and the shared paths. */
export function tieSaid(tie: OperatorTie): string {
  const named = tie.kind === "merged" ? `Took in ${tie.other}` : `Kept in touch with ${tie.other}`;
  const waits =
    tie.waits === undefined ? "" : `. Its ${tie.waits} group waits on ${tie.other}'s push`;
  return `${named}. Shared paths ${tie.paths}${waits}`;
}

/** A standing order in its enforced form. */
export function orderSaid(order: StandingOrder): string {
  switch (order.kind) {
    case "role_off":
      return `${ROLES.find((one) => one.role === order.role)?.name ?? order.role} read-only in ${order.repository}`;
    case "hold_paths":
      return `Hold ${order.paths} until ${order.until.job} ${order.until.event === "ends" ? "ends" : "pushes"}`;
    case "always_raise":
      return `Always raise ${order.what}`;
  }
}

/** How long ago, in the Log's age column: 40s, 3m, 2h. */
export function ageOf(at: string, now: number): string {
  const seconds = Math.max(0, Math.round((now - Date.parse(at)) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.round(minutes / 60)}h`;
}
