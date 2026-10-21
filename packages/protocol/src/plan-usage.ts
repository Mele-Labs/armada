// The plan's windows on the wire, mirrored by hand from `crates/ipc/src/plan_usage.rs`.

/** One window of the account's plan, as a session's mod last read it. */
export type PlanWindow = {
  /** `five_hour`, `seven_day`, or a gateway's `spend_limit`. Draw the two known; leave the rest. */
  kind: string;
  /** How much of the window is used, whole percent. Past 100 on an exceeded spend limit. */
  percent_used: number;
  /** When the window resets. Absent where the reading carried none. */
  resets_at?: string;
};

/**
 * `GET /plan_usage`, and `plan_usage.changed` published whole: the newest reading of the account's
 * windows, from whichever session reported last. Empty `windows` with a `read_at` is an account off a
 * subscription: a session reported none. Without one, none has reported since Fleet began.
 */
export type PlanUsage = {
  windows: PlanWindow[];
  /** When Fleet took the reading. Absent while no session has reported. */
  read_at?: string;
};
