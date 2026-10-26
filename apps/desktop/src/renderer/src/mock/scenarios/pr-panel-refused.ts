// A pull request the forge refused: #1899's page comes back as Fleet's words, the forge's reason and the step that fixes it, so
// the panel says that and not only that the forge did not answer. Every other read is `shared-side-panel`'s. The walk
// `pr-panel-says-why-refused` plays it.

import type { Scenario } from "../moment";
import { s263SharedSidePanel as base } from "./shared-side-panel";

const REFUSED = 1899;
const REFUSAL = "GitHub refused the token gh signed in with. Sign in again with gh auth refresh -s repo, authorize it for the organization's SSO, then press this pull request again.";

export const s264PrPanelRefused: Scenario = {
  ...base,
  name: "pr-panel-refused",
  says: "A pull request the forge refused, with Fleet's reason and what to do",
  draft: {
    ...base.draft,
    sharedPanel: (...args) => {
      const reads = base.draft!.sharedPanel!(...args);
      return { ...reads, pullRequest: (item) => (item.number === REFUSED ? Promise.resolve({ refused: REFUSAL }) : reads.pullRequest(item)) };
    },
  },
};
