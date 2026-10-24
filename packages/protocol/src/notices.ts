/** Every word a Fleet notice goes by and a person can turn off, which a thread row carries. `crates/ipc/src/notices.rs`. */
export const NOTICES = [
  "behind_base",
  "pr_checks_failed",
  "pr_conflicts",
  "pr_dropped",
  "pr_unmergeable",
  "pr_merged",
  "pr_auto_merge",
  "pr_queued",
  "pr_handed",
  "slot_free",
  "slot_kept",
  "build_moved",
  "write_clash",
  "slot_stranded",
  "main_red",
  "rail_nudge",
  "rail_step",
  "usage_trip",
  "limit_wake",
  "plan_approved",
  "auto_land_refused",
  "retro_note",
  "reply_claim",
  "context_handoff",
  "walk_failed",
  "workflow_changed",
  "delivery_asked",
  "delivery_chosen",
  "watch_saw",
  "taught",
] as const;

export type Notice = (typeof NOTICES)[number];

/** Whether a row's `notice` is a word Bridge knows how to mute. */
export const isNotice = (word: unknown): word is Notice => typeof word === "string" && (NOTICES as readonly string[]).includes(word);

/** What a mute covers. `hour` is one Job for an hour from the press. */
export type NoticeScope =
  | { kind: "job"; session_id: string }
  | { kind: "repo"; manifest_id: string }
  | { kind: "hour"; session_id: string };

/**
 * The body and answer of `set_notice_mute` (`POST /notices/mute`). `muted: false` lifts the mute.
 * Fleet keeps mutes on disk and stops sending the notice while one holds. `crates/ipc/src/notices.rs`.
 */
export type SetNoticeMute = {
  notice: Notice;
  scope: NoticeScope;
  muted: boolean;
};
