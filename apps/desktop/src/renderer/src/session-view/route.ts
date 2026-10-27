// The route a Job's delivery takes, read before anything has shipped: every step the kind passes through and where each stands now. A step is
// pending until the data for it arrives (a pull request attached, its Checks, the queue, the merge), then lights as working or done.
// The kind is `landingOf` (`story.ts`); nothing here is guessed from what an agent said.

import type { TaskMarkState } from "@armada/components";
import { attachmentsOf, type Session, type WorkLanding } from "@armada/screens/src/draft/sessions";

import { checksOf } from "./IssuesBlock";

export type RouteState = Extract<TaskMarkState, "open" | "working" | "done" | "failed">;
/** `system` is a step Armada names after something it reports, `ci` or `main`: it is drawn as a reading, not as a word. */
export type RouteStep = { key: string; label: string; state: RouteState; system?: true };

/** What the Job has made, as the panel finds it: whether a document or report exists, and the mods with whether each is on. */
export type Made = { doc: boolean; mods: readonly { enabled: boolean }[] };

const SYSTEM = new Set(["ci", "main"]);
const steps = (labels: readonly string[], states: readonly RouteState[]): RouteStep[] => labels.map((label, at) => ({ key: label, label, state: states[at] ?? "open", ...(SYSTEM.has(label) ? { system: true as const } : {}) }));

export function routeOf(session: Session, kind: WorkLanding, made: Made): RouteStep[] {
  if (kind === "spike") return steps(["Findings"], [made.doc ? "done" : "open"]);
  if (kind === "document" || kind === "report") return steps(["Document", "Read"], [made.doc ? "done" : "open"]);
  if (kind === "mod") {
    const checks = session.rows.filter((row) => row.kind === "check");
    const last = checks[checks.length - 1];
    const check: RouteState = last === undefined || last.kind !== "check" ? "open" : last.state === "running" ? "working" : last.state === "passed" ? "done" : "failed";
    return steps(["Mod", "Check", "On"], [made.mods.length > 0 ? "done" : "open", check, made.mods.some((one) => one.enabled) ? "done" : "open"]);
  }
  const prs = attachmentsOf(session, "pull_request");
  const built = prs.filter((pr) => pr.state !== "draft");
  const merged = prs.length > 0 && prs.every((pr) => pr.state === "merged");
  const pull: RouteState = prs.length === 0 ? "open" : built.length === 0 ? "working" : "done";
  const open = built.filter((pr) => pr.state !== "merged");
  const ci: RouteState = merged ? "done" : built.length === 0 ? "open" : built.some((pr) => checksOf(pr) === "failed") ? "failed" : built.some((pr) => checksOf(pr) === "running") ? "working" : "done";
  const queue: RouteState = merged ? "done" : open.some((pr) => pr.queued === true || pr.auto) ? "working" : "open";
  return steps(["Pull request", "ci", "Merge queue", "main"], [pull, ci, queue, merged ? "done" : "open"]);
}
