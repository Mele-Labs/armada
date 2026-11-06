// The token rules Fleet tells a Job about and the Jobs it told, mirrored by hand from
// `crates/ipc/src/usage_rules.rs`. Names and counts only: a trip names its Job by id and a client
// joins the title it holds. `GET /usage/rules?since=&until=`.

/** `context`. */
export type UsageRuleName = "context";

/** A rule's limit as settings.json holds it. */
export type UsageRuleLimit = {
  value: number;
  /** What `value` counts: `thousand tokens`, `percent` or `turns`. */
  unit: string;
};

/** A Job told that it crossed a rule. */
export type UsageRuleTrip = {
  session_id: string;
  /** RFC3339: the newest turn when the Job was told. */
  at: string;
  /** What crossed: tokens for `context`. */
  figure: number;
  /** Cost units per hour of the Job's active time over the 60 minutes up to `at`, counted as `burn_per_hour_after` is. */
  burn_per_hour_before: number;
  /**
   * Cost units per hour of the Job's active time over the 60 minutes after `at`. Left out until the
   * Job's turns cover that hour, and after, where it made fewer than 5 turns in it.
   */
  burn_per_hour_after?: number;
};

export type UsageRule = {
  rule: UsageRuleName;
  on: boolean;
  limit: UsageRuleLimit;
  /** In the window, newest first. Empty for a rule that never tripped. */
  trips: UsageRuleTrip[];
};

/** The rules, `context`, over `[since, until)`. */
export type UsageRules = {
  since: string;
  until: string;
  rules: UsageRule[];
};
