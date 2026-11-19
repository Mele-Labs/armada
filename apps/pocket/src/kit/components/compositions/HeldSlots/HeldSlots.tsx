/** Why Free is off for a slot: its holder is running, or holds work no commit has. A committed branch is safe, pushed or not. */
export type HeldKeeps = "running" | "uncommitted";

export type HeldSlotRow = {
  slot: number;
  /** The Job that holds it. Its name opens it. */
  job: {
    id: string;
    title: string;
  };
  /** The instant the holder last did anything, as RFC 3339. */
  quietSince: string;
  /** Absent where Free is on. */
  keeps?: HeldKeeps;
};

export type HeldSlotsProps = {
  rows: readonly HeldSlotRow[];
  /** Opens a holder's Job. */
  onOpen: (jobId: string) => void;
  /** Gives the slot to the Job the call is on. A promise keeps the row's Free pending, and every other off, until it settles. */
  onFree: (slot: number) => void | Promise<unknown>;
  /** Makes one more slot for the Job the call is on, as the last row. Absent draws no such row. */
  onCreate?: () => void | Promise<unknown>;
  /** The instant Fleet began making that slot, as RFC 3339, while it is making it. Comes from the call, so leaving and coming back keeps the wait. */
  creatingSince?: string;
  /** Why the last create failed, in Fleet's words. */
  createRefused?: string;
  now?: number;
};
