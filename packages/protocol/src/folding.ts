// What a refused read says — the one place it is decided, for Bridge's main
// process and for anything replaying a recorded answer.
//
// **Pure, and it imports nothing**, which is this package's rule. The sockets,
// the retries and which answer is current stay in main, beside the connection.

import type { WireError } from "./protocol";
import type { Outcome } from "./reads";

/** The request an unreadable answer names, so the fault can say which. */
export type AskedRoute = { method: "GET" | "POST"; path: string; sent?: Record<string, string> };

/**
 * What an answer outside 2xx comes to.
 *
 * **A body that is a `WireError` is Fleet refusing**, and says so in its own
 * code. Anything else is reported as the transport failure it is, with the
 * status, rather than guessed at.
 */
export function refusedWith(status: number, text: string, asked: AskedRoute): Outcome {
  const error = wireErrorIn(text);
  return error === null
    ? {
        ok: false,
        why: "transport",
        detail: `Fleet answered ${status}`,
        fault: { ...asked, why: "unanswerable", status },
      }
    : { ok: false, why: "refused", error };
}

/**
 * A refusal, as the wire carries it, or `null` where the body is not one.
 *
 * **Nothing here mints a code.** A code's declaration lives beside the variant
 * that raises it and `cargo xtask verify-error-codes` collects them across both
 * languages, so a code invented here would be collected from nowhere and mean
 * nothing to the lookup Bridge does.
 */
function wireErrorIn(text: string): WireError | null {
  try {
    const parsed = JSON.parse(text) as WireError;
    if (typeof parsed.code === "string" && typeof parsed.message === "string") return parsed;
  } catch {
    return null;
  }
  return null;
}
