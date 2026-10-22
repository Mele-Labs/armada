// A hosted Job's MCP servers, read from its process and acted on there. `crates/ipc/src/session_mcp.rs`,
// `docs/practices/protocol/a-jobs-mcp-servers.md`. Since 21 Oct 2026.
//
// The header rules in `protocol.ts` hold here. Nothing is kept or published: the process is asked each
// time, so Bridge reads again after each act and when the Loadout opens.

/** Where one server stands, in the harness's five words. */
export type McpServerState = "connected" | "failed" | "needs_sign_in" | "connecting" | "off";

/** One server a hosted Job's process has. */
export type SessionMcpServer = {
  name: string;
  state: McpServerState;
  /** Why it failed, in the harness's words; also a state word Armada does not know. */
  error?: string;
  /** Where it was configured: `project`, `user`, `dynamic` and the like. */
  scope?: string;
  /** The tools it offers; 0 until it is connected. */
  tools: number;
};

/**
 * `GET /sessions/mcp?session_id=`: the servers, as the process answered just now.
 * 422 `fleet.session_mcp_unreachable` for a terminal Job or one with no process running, 422
 * `fleet.session_mcp_refused` where the process refused or did not answer, 409 `fleet.session_closed`.
 */
export type SessionMcp = { servers: SessionMcpServer[] };

/** What a person can do to one server. */
export type McpServerAct = "reconnect" | "turn_on" | "turn_off" | "sign_in";

/** `POST /sessions/mcp/act`. Answered with `SessionMcpActed`, refused as `SessionMcp` is. */
export type ActOnSessionMcp = { session_id: string; server: string; act: McpServerAct };

/** Where a person signs a server in. */
export type McpSignIn = {
  url: string;
  /** The process reconnects the server itself once the person is done. `false`: ask `reconnect` after. */
  callback_expected: boolean;
};

/** The servers after the act, and where to sign in when a sign-in needs the person. */
export type SessionMcpActed = {
  servers: SessionMcpServer[];
  /** Absent for every act but `sign_in`, and for a server signed in already. */
  sign_in?: McpSignIn;
};
