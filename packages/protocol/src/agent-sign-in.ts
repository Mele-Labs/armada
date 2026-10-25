// The agents' account signed out, for the whole Fleet, mirrored by hand from `crates/ipc/src/agent_sign_in.rs`.

/**
 * `GET /agent/sign_in` and `agent_sign_in.changed` published whole. Every Job shares the account, so
 * this names no Job. Absent `signed_out_since` is signed in; absent `signing_in` is false.
 */
export type AgentSignIn = {
  /** When the first Job was heard signed out. */
  signed_out_since?: string;
  /** Fleet is running the sign-in now, waiting on the browser. */
  signing_in?: boolean;
};
