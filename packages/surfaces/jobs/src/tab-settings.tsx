// Settings: the sixth destination, and the only one that changes the Job
// rather than reading it.
//
// **It was a button in the header and a sheet behind it** until the owner
// reversed his own 21 September decision on the 28th: *"I hate that the job
// settings are button. It should be part of the segment control where
// overview, workflow, plan, record, and pulse are now."* What moved is the way
// in; `settings.tsx` still holds every reading and every act, so the panel
// itself did not change hands.
//
// **This file is the destination and nothing else**, on `tab-proposal.tsx`'s
// arrangement: the panel is the reading, and the tabpanel around it is what
// makes the strip's sixth entry land somewhere.

import type { ReactNode } from "react";

import { SettingsPanel, type SettingsPanelProps } from "./settings";
import { TAB_LABEL } from "./detail-tabs";

export type SettingsTabProps = SettingsPanelProps & {
  /**
   * The values that froze at approval, above the ones still open. **A frozen
   * setup is settings** — the owner's 29 September call, which took them off
   * Overview so that destination could say what needs you.
   */
  frozen?: ReactNode;
};

export function SettingsTab({ frozen, ...props }: SettingsTabProps) {
  return (
    <div
      className="armada-detail-tab armada-settings-tab"
      role="tabpanel"
      aria-label={TAB_LABEL.settings}
    >
      {frozen}
      <SettingsPanel {...props} />
    </div>
  );
}
