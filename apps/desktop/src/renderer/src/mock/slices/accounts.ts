// The account and the shared home: the wire is answered by Fleet alone, so the mock has none for it. The walks' own
// sign-in and home are `mock/account.tsx` and `mock/shared-home.tsx`, provided beside the sources rather than through the bridge.

import type { AccountsApi } from "../../../../shared/api/accounts";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const accounts: Slice<AccountsApi, Record<string, never>> = {
  name: "accounts",
  state: {},
  api: () => ({
    getAccounts: async () => ({ ok: false, outcome: unanswered("/accounts") }),
    startSignIn: async () => ({ ok: false, outcome: unanswered("/accounts/start_sign_in") }),
    signOut: async () => ({ ok: false, outcome: unanswered("/accounts/sign_out") }),
    setRepositoryProfile: async () => ({ ok: false, outcome: unanswered("/accounts/set_repository_profile") }),
    getSharedHome: async () => ({ ok: false, outcome: unanswered("/shared_home") }),
    syncSharedHome: async () => ({ ok: false, outcome: unanswered("/shared_home/sync") }),
    addToSharedHome: async () => ({ ok: false, outcome: unanswered("/shared_home/add") }),
    bringFromSharedHome: async () => ({ ok: false, outcome: unanswered("/shared_home/bring") }),
    chooseSharedHomeSide: async () => ({ ok: false, outcome: unanswered("/shared_home/choose") }),
    getHomeKinds: async () => ({ ok: false, outcome: unanswered("/shared_home/kinds") }),
    setHomeKind: async () => ({ ok: false, outcome: unanswered("/shared_home/kinds/set") }),
  }),
};
