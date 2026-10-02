import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { JobDetailField } from "../compositions/JobDetailHeaderActions/JobDetailHeaderActions";

/**
 * What the three job detail renders share.
 *
 * Declared once because all three pass the same block to
 * `JobDetailHeaderActions` — three copies of this type is how the three
 * screens start disagreeing about what a Job's header is.
 */
export type JobDetailHeading = {
  /** The status token stem, e.g. `running`. From the generated vocabulary. */
  status: string;
  statusIcon: LucideIcon;
  statusLabel: ReactNode;
  headline: ReactNode;
  jobId?: ReactNode;
  /** The whole identifier, where what is drawn is short for it. Hovered and copied. */
  jobIdWhole?: string;
  /** What the id is called. Absent draws it alone. */
  jobIdLabel?: ReactNode;
  fields: JobDetailField[];
  /** The controls at the header's trailing edge. `Kill`, or a redispatch. */
  actions?: ReactNode;
  /**
   * A fact carrying an `href` was clicked, with the address it carries.
   *
   * **Here rather than beside `onCopied` on the screen**, and the difference is
   * scope: copying is offered by every mono value on job detail, in four
   * regions, so the screen owns it; a link is offered by one fact in this
   * block, and the address and what to do with it are one decision made in one
   * place. Riding on the heading means it reaches the header through the same
   * spread the fields do, and no render in between has to know it exists.
   */
  onFollowed?: (href: string) => void;
  /**
   * A fact carrying an `opensJob` was pressed, with the Job id it names.
   *
   * Rides on the heading for `onFollowed`'s reason, and is the inward half of
   * the same pair: one fact in this block points at another Job, and where
   * that press lands is one decision made in one place.
   */
  onOpenJob?: (jobId: string) => void;
};

