// A Job's walk drawn in a pane of Bridge's own window: its page and its bar are views main lays over the frame the pane
// reports (`main/capture/job-walks.ts`). A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
// Every call names a Job and the address of a walk it showed. **None carries an address to load**: main holds the page.

import type { Outcome } from "@armada/protocol";
import type { PageBounds } from "@armada/screens/src/draft/sessions";
import type { PanelHeldNote } from "@armada/studios/api";

/** The walk's page and its bar as pictures, held still while a layer of Bridge's own lies over the frame. */
export type JobWalkFrozen = {
  page: Uint8Array | null;
  bar: Uint8Array | null;
};

export type JobWalkApi = {
  /**
   * **Absent in the mock**, whose `WalkPane` draws the walk itself. The pane reports its frame as the browser pane does; the page and the bar
   * of the walk it names are views main lays inside it, and parks hidden while no pane reports one.
   */
  jobWalk?: {
    place: (sessionId: string, walkId: string, bounds: PageBounds) => Promise<void>;
    unplace: (sessionId: string, walkId: string) => Promise<void>;
    /** Both views hidden and their pictures returned, while a menu, sheet or drag of Bridge's lies over the frame. `null`: no walk by that name. */
    freeze: (sessionId: string, walkId: string) => Promise<JobWalkFrozen | null>;
    thaw: (sessionId: string, walkId: string) => Promise<void>;
    /** The notes held on a Job, told as they change and asked for once. Main sends only what the Job holds. */
    held: (sessionId: string, onChanged: (notes: PanelHeldNote[]) => void) => () => void;
    editNote: (sessionId: string, id: string, said: string) => Promise<Outcome>;
    removeNote: (sessionId: string, id: string) => Promise<Outcome>;
    /** Every held note as one turn, as the bar's Send notes sends them. */
    sendNotes: (sessionId: string) => Promise<Outcome>;
  };
};

export const JOB_WALK_CHANNELS = {
  jobWalkPlace: "bridge:job-walk-place",
  jobWalkUnplace: "bridge:job-walk-unplace",
  jobWalkFreeze: "bridge:job-walk-freeze",
  jobWalkThaw: "bridge:job-walk-thaw",
  jobWalkHeld: "bridge:job-walk-held",
  jobWalkHeldChanged: "bridge:job-walk-held-changed",
  jobWalkEditNote: "bridge:job-walk-edit-note",
  jobWalkRemoveNote: "bridge:job-walk-remove-note",
  jobWalkSendNotes: "bridge:job-walk-send-notes",
} as const;
