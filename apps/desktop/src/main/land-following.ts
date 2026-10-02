// The socket that reads one merge line Check's log, running or ended — `following.ts` one subject
// over. `observe_land_check`, protocol 23.7.
//
// **Its own class rather than `FollowSocket` widened.** A Job's Check is named by its Job and the
// file Fleet keeps it in; a merge line Check is named by the line's three words and has no Job, so
// the two readings differ in every field they are keyed by. What they share is the shape of the
// reading, and the window: `HELD` lines, the same as a Job's Check.
//
// **One at a time**, and it reads and cannot do anything else, for `following.ts`'s reasons.

import WebSocket from "ws";

import type { FollowedLandLog, LandCheckAt, LandOutputMessage } from "@armada/protocol";
import { HOST } from "./runtime-file";

/** How many lines main holds for the reader: `following.ts`'s window. */
const HELD = 2_000;

export class LandFollowSocket {
  private readonly publish: (landFollowed: FollowedLandLog) => void;
  private socket: WebSocket | null = null;
  private held: FollowedLandLog = { state: "none" };

  constructor(publish: (landFollowed: FollowedLandLog) => void) {
    this.publish = publish;
  }

  /**
   * Which Check's log is being read, or `null` to stop. **The same Check asked for again is the
   * same reading**, `following.ts`'s rule.
   */
  open(port: number | null, at: LandCheckAt | null): void {
    const held = this.held;
    if (at !== null && held.state !== "none" && held.state !== "failed" && same(held, at)) return;
    this.close();
    if (at === null) {
      this.set({ state: "none" });
      return;
    }
    const asked = { root: at.root, branch: at.branch, check: at.check };
    if (port === null) {
      this.set({ state: "failed", ...asked, detail: "Fleet is not connected." });
      return;
    }
    this.set({ state: "opening", ...asked });

    const query = new URLSearchParams(asked).toString();
    const socket = new WebSocket(`ws://${HOST}:${port}/merge_lines/checks/observe?${query}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(asked, String(data)));
    socket.on("error", (cause: Error) => this.broke(asked, cause.message));
    socket.on("close", () => this.broke(asked, "the connection closed"));
  }

  close(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, for `journal.ts`'s reason.
    socket.on("error", () => {});
    socket.close();
  }

  private set(landFollowed: FollowedLandLog): void {
    this.held = landFollowed;
    this.publish(landFollowed);
  }

  private arrived(at: LandCheckAt, text: string): void {
    let message: LandOutputMessage;
    try {
      message = JSON.parse(text) as LandOutputMessage;
    } catch {
      this.broke(at, "Fleet sent a message this Bridge could not read.");
      return;
    }
    if (message.message === "opened") {
      this.set({ state: "following", ...at, fromLine: message.skipped + 1, lines: [] });
      return;
    }
    const held = this.held;
    if (held.state !== "following") return;
    if (message.message === "lines") {
      const all = [...held.lines, ...message.lines];
      const dropped = Math.max(0, all.length - HELD);
      this.set({ ...held, lines: all.slice(dropped), fromLine: held.fromLine + dropped });
      return;
    }
    // Let go first, so the socket's own `close` cannot overwrite the reason.
    this.close();
    this.set({ ...held, ended: message.because });
  }

  /** The socket went without a sentence. What had arrived stays arrived. */
  private broke(at: LandCheckAt, detail: string): void {
    if (this.socket === null) return;
    this.socket.removeAllListeners();
    this.socket = null;
    const held = this.held;
    if (held.state === "following") {
      this.set({ ...held, ended: held.ended ?? "broke" });
      return;
    }
    this.set({ state: "failed", ...at, detail });
  }
}

/**
 * The three names off what a window sent, and nothing else; `null` for anything that is not three
 * strings. **Main's own check at the IPC seam**: a renderer is not trusted to send the shape its
 * type says.
 */
export function landCheckAt(sent: unknown): LandCheckAt | null {
  if (typeof sent !== "object" || sent === null) return null;
  const { root, branch, check } = sent as Record<string, unknown>;
  if (typeof root !== "string" || typeof branch !== "string" || typeof check !== "string") return null;
  return { root, branch, check };
}

function same(one: LandCheckAt, other: LandCheckAt): boolean {
  return one.root === other.root && one.branch === other.branch && one.check === other.check;
}
