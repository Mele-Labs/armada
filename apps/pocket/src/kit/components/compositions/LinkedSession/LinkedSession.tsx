import type { SessionState } from "../SessionFrame/SessionFrame";
import type { LedgerPullRequest } from "../SessionLedger/PullRequestMarks";

/**
 * A Session a parent follows, as one press that opens it, in two lines: the `link` glyph and the title, then in the muted mono sub-line the plan
 * group it was handed from where the row is not already under that group, its state as the one registry mark, and the pull request it holds.
 * **Drawn the same wherever the parent keeps its children**, on a Plan group or in the Ledger's Linked section.
 */
export type LinkedSessionProps = {
  title: string;
  /** The plan group's title, when the row does not stand under it. */
  group?: string;
  state: SessionState;
  /** The state's name, which the mark's tooltip says. */
  said: string;
  pr?: LedgerPullRequest & {
    number: number;
  };
  onOpen: () => void;
};
