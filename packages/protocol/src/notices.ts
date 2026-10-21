/** A Fleet notice a person can turn off. */
export type Notice = "behind_base";

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
