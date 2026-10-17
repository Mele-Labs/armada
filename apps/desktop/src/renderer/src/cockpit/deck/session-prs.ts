// Which pull requests the Merge line marks while a Session is open: the ones it owns or opened. A pull request is the Session's when the
// forge listing named it the owner (`Pr.owner`), or when the Session holds it as an attachment, by number or by its branch, as `ship.ts`
// names an owner. Nothing selected marks none.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Pr } from "./ship";

export function sessionHolds(session: Pick<Session, "id" | "attachments">, pr: Pick<Pr, "number" | "branch" | "owner">): boolean {
  if (pr.owner?.kind === "session" && pr.owner.id === session.id) return true;
  return session.attachments.some((one) => (one.kind === "pull_request" && (one.number === pr.number || one.branch === pr.branch)) || (one.kind === "branch" && one.name === pr.branch));
}
