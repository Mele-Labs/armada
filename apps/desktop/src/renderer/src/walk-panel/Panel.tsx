// The panel's content, apart from the entry that mounts it, so a test draws it on the mock Fleet.

import type { WalkSubject } from "./subject";
import { WalkSession } from "./WalkSession";

/** The Session this window walks. */
export function Panel({ subject }: { subject: WalkSubject | null }) {
  if (subject === null) return null;
  return <WalkSession sessionId={subject.id} />;
}
