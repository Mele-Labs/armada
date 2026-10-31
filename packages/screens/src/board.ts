// What the picked repository lists: its Sessions, and the slots Fleet holds disk for.

import type { SessionRecord, WorktreesHeld } from "@armada/protocol";

/**
 * The Sessions the pick names. `null` is no one repository — All repositories, or a Fleet that lists
 * none — and scopes nothing. One with no Manifest of its own is on All alone, **except Setup's, which
 * stands in the root of the repository being set up**: that repository has no Manifest either, so the
 * root is what places it, before the file it writes is loaded and after.
 */
export function sessionsOfPicked(
  sessions: readonly SessionRecord[],
  picked: { root?: string; manifest?: { id: string }; implicit_id?: string } | null,
): readonly SessionRecord[] {
  if (picked === null) return sessions;
  return sessions.filter((one) =>
    one.manifest_id === undefined
      ? picked.root !== undefined && one.cwd === picked.root
      : one.manifest_id === (picked.manifest?.id ?? picked.implicit_id),
  );
}

/** What Fleet holds disk for, of the picked repository: a slot names its Manifest. */
export function heldOfPicked(held: WorktreesHeld, picked: { manifest?: { id: string }; implicit_id?: string } | null): WorktreesHeld {
  if (picked === null) return held;
  return held.slots === undefined ? held : { slots: held.slots.filter((one) => one.manifest_id === (picked.manifest?.id ?? picked.implicit_id)) };
}

/** The repositories a Workspace holds, as the pages list them. */
type Member = { root?: string; manifest?: { id: string }; implicit_id?: string };

/** The Sessions of every repository the Workspace holds, placed as `sessionsOfPicked` places one. */
export function sessionsOfWorkspace(sessions: readonly SessionRecord[], members: readonly Member[]): readonly SessionRecord[] {
  return sessions.filter((one) => members.some((member) => sessionsOfPicked([one], member).length > 0));
}

/** What Fleet holds disk for, of every repository the Workspace holds. */
export function heldOfWorkspace(held: WorktreesHeld, members: readonly Member[]): WorktreesHeld {
  if (held.slots === undefined) return held;
  const ids = members.map((one) => one.manifest?.id ?? one.implicit_id);
  return { slots: held.slots.filter((one) => ids.includes(one.manifest_id)) };
}
