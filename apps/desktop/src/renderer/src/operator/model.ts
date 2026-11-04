// What the Operator panel draws, typed here because no wire carries it yet. Each shape is a proposal for the protocol, listed in the
// header of the walks `operatorWide` and `operatorFull` (`mock/walks/operator.ts`). The mock scenario seeds it; the panel's own presses move it locally.

/** What the Operator did with an item, the Log's third column. */
export type OperatorDid =
  | "answered"
  | "raised"
  | "delegated"
  | "held"
  | "started"
  | "filed"
  | "passed_on"
  | "released"
  | "returned"
  | "resumed"
  | "moved";

/** The four roles a person turns. Evaluate is the Operator switch itself; Raise has no control. */
export type OperatorRole = "dispatch" | "delegate" | "answer" | "file";

export type OperatorPosition = "on" | "suggest" | "off";

/** What a held item waits for. Fleet refuses a hold that names neither a Job nor a meter; `you` is a row the person moved there. */
export type OperatorWait =
  | {
      kind: "job";
      job: string;
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

/** One act, newest first. */
export type OperatorLogRow = {
  id: string;
  /** ISO time it was ruled. */
  at: string;
  /** Where the item came from: a Job's title, or what arrived, such as "You dispatched" or "Bug report". */
  from: string;
  /** The item itself: a question, a request, a report. */
  item: string;
  did: OperatorDid;
  why: string;
  /** The Line row a hold put on the Line, which Start now on this row starts. */
  line?: string;
  /** The other ways Fleet offered for this item, which Change picks from. Absent draws no Change. */
  ways?: readonly string[];
  /** The way the person changed it to, a correction the next call reads. */
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

/** How the panel is drawn, the two takes the owner picks between. */
export type OperatorTake = "wide" | "full";

export type OperatorSeed = {
  take: OperatorTake;
  /** The Operator switch: off sends everything to the person as today. */
  on: boolean;
  positions: Readonly<Record<OperatorRole, OperatorPosition>>;
  /** The most calls a day before it stops and everything passes through. */
  ceiling: number;
  log: readonly OperatorLogRow[];
  line: readonly OperatorLineRow[];
  orders: readonly StandingOrder[];
  reads: readonly OperatorRead[];
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
  held: "Held",
  started: "Started",
  filed: "Filed",
  passed_on: "Passed on",
  released: "Released",
  returned: "Returned",
  resumed: "Resumed",
  moved: "Moved",
};

const METER_SAID = {
  five_hour: "5 hour window",
  weekly: "Weekly window",
} as const;

/** What a Line row waits for, as its line under the title says it. */
export function waitSaid(wait: OperatorWait): string {
  switch (wait.kind) {
    case "job":
      return wait.event === "ends"
        ? `${wait.job} ends`
        : wait.group === undefined
          ? `${wait.job} pushes`
          : `${wait.job} pushes its ${wait.group} group`;
    case "meter":
      return `${METER_SAID[wait.meter]} under ${wait.under}%`;
    case "you":
      return "You moved it here";
  }
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
