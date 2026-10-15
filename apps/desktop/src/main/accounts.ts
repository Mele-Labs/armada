// The account, as main holds it: four requests to Fleet. Nothing is kept here; the profiles live in
// Fleet and arrive whole, and the renderer decides when to read again.

import type { Accounts, Outcome, SignedOut } from "@armada/protocol";
import type { AccountsActed } from "../shared/api/accounts";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class AccountsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(): Promise<AccountsActed> {
    return await this.act<Accounts>("GET", "/accounts");
  }

  async startSignIn(): Promise<AccountsActed> {
    return await this.act<Accounts>("POST", "/accounts/start_sign_in");
  }

  async signOut(accountId: number): Promise<AccountsActed<SignedOut>> {
    return await this.act<SignedOut>("POST", "/accounts/sign_out", { account_id: accountId });
  }

  private async act<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<AccountsActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as T };
  }
}
