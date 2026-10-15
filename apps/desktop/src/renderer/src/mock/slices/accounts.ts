// The account: the wire is answered by Fleet alone, so the mock has none for it. The walks' own sign-in
// is `mock/account.tsx`, provided beside the sources rather than through the bridge.

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
    setActiveAccount: async () => ({ ok: false, outcome: unanswered("/accounts/set_active_account") }),
  }),
};
