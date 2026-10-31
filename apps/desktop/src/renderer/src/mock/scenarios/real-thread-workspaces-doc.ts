// The Job "Workspace Autonomy Defaults" as it really went, in the new thread look, for the walk `real-thread-workspaces-doc`. Over `sessions`.

import type { Scenario } from "../moment";
import { WORKSPACES_DOC_ROWS } from "../real-threads/workspaces-doc";
import { realThreadScenario } from "../real-threads/real-thread";

export const s341dRealThreadWorkspacesDoc: Scenario = realThreadScenario({
  name: "real-thread-workspaces-doc",
  says: "A real Job's thread with a long back and forth, six answers of his, ending on the agent's open question",
  id: "0dd18323-5a5f-419c-a28e-710dc85f787e",
  title: "Workspace Autonomy Defaults",
  branch: "armada/session-0dd18323",
  slot: 10,
  rows: WORKSPACES_DOC_ROWS,
  asked: ["Yes, on this Mac", "No, with the repo", "Let me look"],
});
