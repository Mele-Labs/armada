// What the Operator's page and the places beside it draw, typed here because no wire carries all of it. The mock scenario seeds it;
// the page's own presses move it locally. `line` is the list of Jobs held to start, which a held Job's header and the Cockpit read.

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
  /** ISO time it was said, for the message's band. */
  at?: string;
  /** A Job this started, which the message opens. */
  opens?: OperatorJob;
  /** A Job the Operator offered to read the code, by its brief: Dispatch starts it in one of `in`, a repository id. */
  offer?: {
    brief: string;
    in: readonly string[];
    left?: boolean;
  };
};

/** An offered Job's title: its brief's first sentence, cut to fit a row. */
export function offerTitle(brief: string): string {
  const first = (brief.trim().split(/(?<=[.?!])\s/)[0] ?? "").replace(/[.]$/, "");
  return first.length <= 72 ? first : `${first.slice(0, 71).trimEnd()}…`;
}

/** The mock's script for the box: what a fresh call answers to words that hold `asked`. */
export type OperatorTalk = {
  asked: string;
  said: string;
  /** The brief of a Job it offers, as Fleet's answer carries one where the code needs reading. */
  offers?: string;
};

/** Something the Operator raised with no Job to stand on, which the Cockpit draws as a row that opens the Operator. */
export type OperatorRaise = {
  id: string;
  item: string;
};

export type OperatorSeed = {
  /** The Operator switch: off sends everything to the person as today. */
  on: boolean;
  line: readonly OperatorLineRow[];
  reads: readonly OperatorRead[];
  ties: readonly OperatorTie[];
  talk: readonly OperatorTalk[];
  raises?: readonly OperatorRaise[];
  /** Every repository a Job the Operator offers could start in. */
  served?: readonly string[];
  /** Where a raise tied to no Job pulses: on the title row's button alone, or on a row of the Jobs list too. */
  raisesPulse?: "button" | "row";
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

/** What a Job row's tie glyph says, naming the other Job and the shared paths. */
export function tieSaid(tie: OperatorTie): string {
  const named = tie.kind === "merged" ? `Took in ${tie.other}` : `Kept in touch with ${tie.other}`;
  const waits =
    tie.waits === undefined ? "" : `. Its ${tie.waits} group waits on ${tie.other}'s push`;
  return `${named}. Shared paths ${tie.paths}${waits}`;
}
