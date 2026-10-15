// The repository in a Session's head, and the move it offers when the Session started in the wrong one.
// **Absent where Fleet does not serve `move`**: the head then says nothing of the repository.

import type { ReactNode } from "react";

import { SessionRepository } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repositoryLabel } from "@armada/shell";

import { useSessionsDraft } from "./sessions-draft";

/** Said on each repository while a terminal is still listening: Fleet cannot move the person's terminal. */
export const IN_A_TERMINAL = "Runs in a terminal that is still listening";

export function SessionRepositoryOf({ session, repositories, onOpen }: { session: Session; repositories: readonly RepositorySummary[]; onOpen: (id: string) => void }): ReactNode {
  const move = useSessionsDraft()?.move;
  const served = repositories.filter((one) => one.manifest !== undefined);
  const here = served.find((one) => one.manifest?.id === session.manifestId);
  if (move === undefined || session.helm === true || here === undefined) return null;
  const branch = session.attachments.find((one) => one.kind === "branch")?.name;
  const holds = branch !== undefined || session.attachments.some((one) => one.kind === "slot");
  return (
    <SessionRepository
      name={repositoryLabel(here, repositories)}
      to={served.filter((one) => one !== here).map((one) => ({ id: one.manifest!.id, name: repositoryLabel(one, repositories) }))}
      {...(session.terminal === true && session.dead === undefined ? { held: IN_A_TERMINAL } : {})}
      {...(holds ? { stays: branch === undefined ? {} : { branch } } : {})}
      ends={session.dead === undefined}
      onMove={async (manifestId) => {
        const next = await move(session.id, manifestId);
        if (next !== undefined) onOpen(next);
      }}
    />
  );
}
