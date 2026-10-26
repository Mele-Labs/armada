// Where a Job's own run goes: its bay, which it holds from its first write. A Job with none has nothing to run.

import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { rootOfSession } from "../../run-places";
import type { RunTarget } from "./source";

export function jobTarget(session: Pick<Session, "id" | "title" | "manifestId" | "cwd" | "attachments">, repositories: readonly RepositorySummary[]): RunTarget | undefined {
  const root = rootOfSession(session, repositories);
  const bay = session.attachments.find((one) => one.kind === "slot");
  if (root === undefined || bay === undefined || bay.kind !== "slot") return undefined;
  const branch = session.attachments.find((one) => one.kind === "branch");
  return { repository: root, job: session.id, checkout: { kind: "job", title: session.title ?? "Job", branch: branch?.kind === "branch" ? branch.name : "", bay: bay.slot } };
}
