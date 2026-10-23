/** A Fleet notice a person can turn off, and the word its thread row carries. `crates/ipc/src/notices.rs`. */
export type Notice =
  | "behind_base"
  | "pr_checks_failed"
  | "pr_conflicts"
  | "pr_dropped"
  | "pr_unmergeable"
  | "pr_merged"
  | "pr_auto_merge"
  | "pr_queued"
  | "slot_free"
  | "slot_kept"
  | "build_moved"
  | "write_clash"
  | "slot_stranded"
  | "main_red"
  | "rail_nudge"
  | "rail_step"
  | "usage_trip"
  | "limit_wake"
  | "plan_approved"
  | "auto_land_refused"
  | "retro_note"
  | "reply_claim"
  | "context_handoff"
  | "walk_failed"
  | "workflow_changed"
  | "delivery_asked"
  | "delivery_chosen"
  | "watch_saw"
  | "taught";

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
