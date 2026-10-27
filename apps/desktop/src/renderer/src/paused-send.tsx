// A note typed to a Job he has parked asks before it goes: nothing but his Resume wakes a paused Job, so the note waits on that.

import { useState, type ReactNode } from "react";

import { Dialog } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

type Held = { go: () => void | Promise<boolean>; settle: (went: boolean) => void };

/**
 * `ask(go)` runs `go` at once for a Job that is not paused. For a paused one it opens the confirm and answers when he has chosen:
 * `false` on Cancel, which gives the box its words back, and what `go` answers after Resume. `layer` is the confirm, to draw once.
 */
export function usePausedSend(draft: Pick<SessionsDraft, "resume">, session: Session): { ask: (go: Held["go"]) => void | Promise<boolean>; layer: ReactNode } {
  const [held, setHeld] = useState<Held>();
  const parked = session.paused === true && draft.resume !== undefined;
  const ask: (go: Held["go"]) => void | Promise<boolean> = (go) => (parked ? new Promise<boolean>((settle) => setHeld({ go, settle })) : go());
  const cancel = () => {
    held?.settle(false);
    setHeld(undefined);
  };
  const confirm = async () => {
    const one = held;
    setHeld(undefined);
    if (one === undefined) return;
    // The note goes only to a Job that took its slot again.
    if (!(await draft.resume!(session.id))) return one.settle(false);
    one.settle((await one.go()) ?? true);
  };
  const layer = (
    <Dialog open={held !== undefined} title="Job is paused" tone="neutral" confirmLabel="Resume and send" initialFocus="confirm" onConfirm={() => void confirm()} onCancel={cancel}>
      Resuming takes a slot again. The note goes after.
    </Dialog>
  );
  return { ask, layer };
}
