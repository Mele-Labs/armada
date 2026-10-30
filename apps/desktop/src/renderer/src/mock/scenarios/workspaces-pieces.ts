// The Workflows page over the lifecycles and rules a Fleet keeps for store-web, the first repository of the Workspace `my-store`: Armada's Ticket, Pull request and Deploy, a Ticket of the
// repository's own, and three rules, one of the person's, one the repository committed and one for the Workspace. The page reads them as it reads a real Fleet's and saves a change as the
// pieces it changes (`slices/lifecycles.ts`). The Workspace of `workspaces-lanes` is the page's own mock; this one is Fleet's. The walk `workspaces-pieces` plays it.

import { storePieces } from "../pieces-seed";
import type { Scenario } from "../moment";
import { workspacesLanes } from "./workspaces-lanes";

const [web, api] = workspacesLanes.state.holds.repositories ?? [];

export const workspacesPieces: Scenario = {
  name: "workspaces-pieces",
  says: "The Workflows page over the lifecycles and rules Fleet keeps for store-web: edited by hand, each change saved as the pieces it changes",
  state: workspacesLanes.state,
  ...(workspacesLanes.behaves === undefined ? {} : { behaves: workspacesLanes.behaves }),
  draft: { pieces: storePieces({ id: web!.manifest!.id, root: web!.root }, { id: api!.manifest!.id, root: api!.root }) },
};
