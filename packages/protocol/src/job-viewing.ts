// When the person was last in a Job, told by the client that has it on screen. Mirrors `crates/ipc/src/job_viewing.rs`.

/** `view_session` (`POST /sessions/view`): the Job came on screen in a visible window (`on`), or left it, or the window hid. */
export type ViewSession = {
  session_id: string;
  on: boolean;
};
