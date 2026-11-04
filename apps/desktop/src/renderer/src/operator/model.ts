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

export type OperatorPosition = "on" | "suggest" | "off";

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

/** One message in the box's conversation. */
export type OperatorMessage = {
  id: string;
  from: "you" | "operator";
  text: string;
};

/** The mock's script for the box: what a fresh call answers to words that hold `asked`, and the Line row it starts, if any. */
export type OperatorTalk = {
  asked: string;
  said: string;
  starts?: string;
};

export type OperatorSeed = {
  /** The Operator switch: off sends everything to the person as today. */
  on: boolean;
  positions: Readonly<Record<OperatorRole, OperatorPosition>>;
  /** The most calls a day before it stops and everything passes through. */
  ceiling: number;
  log: readonly OperatorLogRow[];
  line: readonly OperatorLineRow[];
  orders: readonly StandingOrder[];
  reads: readonly OperatorRead[];
  ties: readonly OperatorTie[];
  talk: readonly OperatorTalk[];
};

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
  on: "On",
  suggest: "Suggest",
  off: "Off",
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
  passed_on: "Passed on",
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
      return `${ROLES.find((one) => one.role === order.role)?.name ?? order.role} off in ${order.repository}`;
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
