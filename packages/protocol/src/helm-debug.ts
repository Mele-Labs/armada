// One repository's Helm session as one quotable record — `GET /helm/debug`,
// `crates/ipc/src/helm_debug.rs`. #1367.
//
// Hand-written like `helm.ts`, for that file's own reason: the codegen that
// would emit both sides does not exist yet.
//
// **Fields, and the string is Bridge's.** `HelmRecord/record.ts` is the one
// producer that formats these, the way `ErrorNotice/payload.ts` is the one
// producer of an error's payload — what is read on screen is what arrives in
// the issue body, by construction rather than by two renderings agreeing.

import type { Outcome } from "./reads";
import type { HelmActionAuthority } from "./health";
import type { ProtocolId } from "./connection";

/** Everything a person can carry to whoever could fix a bad Helm answer. */
export type HelmDebugInfo = {
  manifest_id: string;
  /** The checkout the session opens in — a folder on this person's own disk. */
  checkout: string;
  /** The Machine setting, resolved: what this session may do. */
  authority: HelmActionAuthority;
  /** The model each message's process is started with. */
  model: string;
  /**
   * The note appended to the Session's system prompt, whole, as the next process is started with it.
   * Was `brief`.
   */
  note: string;
  /** The MCP server Armada's door is registered under, which prefixes every name in `tools` inside the session. */
  door: string;
  /** Every tool the door offered this session, by name, as the inventory orders them. */
  tools: string[];
  /** The Helm Session's id. Absent where the repository has none live: before the first message, and after a start fresh. */
  session?: string;
  /** The thread, oldest first, bounded — `cut` says by how much. */
  thread: HelmDebugLine[];
  /** How many older lines the thread left out. Said rather than implied. */
  cut: number;
  /** What this session's last `get_events_since` was answered. Absent where it has not polled since Fleet started. */
  polled?: HelmDebugPolled;
  /** Which Fleet process answered. Fleet holds no application version, the way Bridge holds none. */
  run_id: string;
  protocol_id: ProtocolId;
  /** When the record was taken, on Fleet's clock. Taken, not raised. */
  at: string;
};

/** One line of the thread, as the record holds it. */
export type HelmDebugLine = { at: string } & HelmDebugSaid;

/**
 * What one line was — a narrowing of the thread's own vocabulary, not a second
 * copy of it: what was asked, what came back, what was called, what was
 * refused, and where words were sent. **`ended`, `fresh` and `unanswered` are gone**: a Session's
 * thread has no such rows.
 */
export type HelmDebugSaid =
  | { line: "asked"; text: HelmDebugText }
  | { line: "said"; text: HelmDebugText }
  | { line: "called"; tool: string; detail: string }
  | { line: "refused"; tool: string; because: string }
  /** Words Helm's dock sent to a Session or a Job, and which. Never told to Helm's process. */
  | { line: "sent"; text: HelmDebugText; to: string };

/** A piece of prose, bounded where the record cut it. */
export type HelmDebugText = {
  text: string;
  /** How long it was before it was cut. Absent where nothing was cut. */
  of?: number;
};

/** `EventsSince` on the Rust side: one poll's answer, counted rather than carried. */
export type HelmDebugPolled = {
  from: number;
  upto: number;
  kinds: { kind: string; count: number }[];
  /** How many events after `from` were dropped before they could be counted. */
  missed?: number;
};

/**
 * What a reading of one session came back as. **`CheckOutputRead`'s shape, and for
 * its reasons**: it is asked for once, by the person who opened the record,
 * and it is theirs — putting it in `BridgeState` would make one reader's
 * gesture part of what every surface re-renders on.
 */
export type HelmDebugRead = { ok: true; record: HelmDebugInfo } | { ok: false; outcome: Outcome };
