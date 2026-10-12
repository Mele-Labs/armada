// Whether the page a walk window shows is ready to walk — `docs/practices/capture-window.md`.
//
// **Read off what Bridge already holds, never asked of the page.** A Session's window is ready unless
// Fleet marks the window's artifact `working` (the Session wrote a file since it showed the page); a
// Job's is ready while the Job waits on the person and working while it runs. Pure, so the rule is
// tested without a window.

import type { SessionRecord } from "@armada/protocol";
import type { CaptureReadiness } from "../../shared/capture-window";

/** The Session's window at `address`: `working` when its artifact says so, else `ready`; `undefined` where the ledger shows no such window. */
export function sessionReadiness(record: SessionRecord | undefined, address: string): CaptureReadiness | undefined {
  const shown = record?.attachments.find(
    (one) => one.kind === "artifact" && one.target === address && one.detail?.["form"] === "window",
  );
  if (shown === undefined) return undefined;
  return shown.detail?.["working"] === "true" ? "working" : "ready";
}

/** A Job's window by the Job's status: waiting on the person is ready, running is working, anything else says nothing. */
export function jobReadiness(status: string | undefined): CaptureReadiness | undefined {
  if (status === "awaiting_review" || status === "awaiting_approval") return "ready";
  return status === "running" ? "working" : undefined;
}
