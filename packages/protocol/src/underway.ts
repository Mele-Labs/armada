// A running Check's log, read as it grows. `crates/ipc/src/underway.rs`.

/**
 * The running Check's log a window is following, as main holds it.
 *
 * **One at a time**, for the journal socket's reason: a person reads one
 * Check's log, and opening another replaces the first.
 */
export type FollowedLog =
  | { state: "none" }
  | { state: "opening"; jobId: string; kept: string }
  | {
      state: "following";
      jobId: string;
      kept: string;
      name: string;
      attempt: number;
      path: string;
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /**
       * Why the stream ended — `finished` or `unreadable` as Fleet said it, or
       * `broke` where the connection went without a sentence — or absent while
       * it is still arriving.
       */
      ended?: string;
    }
  | { state: "failed"; jobId: string; kept: string; detail: string };
