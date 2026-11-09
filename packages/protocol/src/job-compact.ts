// Whether a Job offers Compact, and if not why, read by Fleet so a client does not work it out. States only: the
// words for each reason stay with the client. Carried on `SessionRecord.compact`. Mirrors `crates/ipc/src/job_compact.rs`.

/** How Compact reaches the Job: typed in its terminal as a command, or sent as a message to the agent Fleet hosts. */
export type CompactHow = "terminal" | "message";

/** Why Compact is off, in the order Bridge checks them. */
export type CompactOff = "ended" | "not_listening" | "terminal_not_listed" | "agent_not_listed";

/** Compact offered, or off with the reason. */
export type CompactOffer = { state: "offered"; how: CompactHow } | { state: "off"; why: CompactOff };
