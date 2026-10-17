// Who asked for a Check run. Since 23.40. The Rust half is `crates/ipc/src/requester.rs`.

/**
 * Who asked for one run, as a value a surface can follow to the place that
 * asked.
 *
 * **`kind` is an opaque string**, as a Record row's is: a kind this build has
 * never heard of is drawn from the ids it carries, and is never a type error.
 * The ids each kind needs are beside it, absent on every kind that does not use
 * them.
 *
 * | `kind` | Who | Ids |
 * |---|---|---|
 * | `merge_line` | The merge line, for one branch | `branch` |
 * | `session` | A Session's agent, running `armada check` in the slot it holds. Since 23.75 | `session_id`, `slot` |
 * | `outside` | Nothing in Armada: a person's press, a bare `armada check` | none |
 *
 * **`outside` is a value, and an absent requester reads as it** — a Fleet
 * before 23.40 sends none, and a fixture may not.
 */
export type Requester = {
  kind: string;
  branch?: string;
  /** The Session whose agent ran it, and the slot it ran in. Since 23.75. */
  session_id?: string;
  slot?: number;
};

export const REQUESTER = {
  mergeLine: "merge_line",
  session: "session",
  outside: "outside",
} as const;

/** What a row that carries no requester reads as. */
export const OUTSIDE: Requester = { kind: REQUESTER.outside };

/** The requester a row carries, or `outside` where it carries none. */
export function requesterOf(row: { requester?: Requester }): Requester {
  return row.requester ?? OUTSIDE;
}
