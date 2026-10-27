// The account, as main holds it: requests to Fleet for the profiles and the sign-in, and for each
// profile's shared home and What syncs. Nothing is kept here; both live in Fleet and arrive whole, and
// the renderer decides when to read again.

import type { Accounts, HomeKind, HomeKinds, HomeRowAt, HomeSide, HomeSynced, Outcome, SharedHomeRead, SignedOut } from "@armada/protocol";
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

  async setRepositoryProfile(root: string, accountId: number | null): Promise<AccountsActed> {
    return await this.act<Accounts>("POST", "/accounts/set_repository_profile", { root, account_id: accountId });
  }

  async sharedHome(login: string): Promise<AccountsActed<SharedHomeRead>> {
    return await this.act<SharedHomeRead>("GET", `/shared_home?login=${encodeURIComponent(login)}`);
  }

  async syncSharedHome(login: string): Promise<AccountsActed<HomeSynced>> {
    return await this.act<HomeSynced>("POST", "/shared_home/sync", { login });
  }

  async addToSharedHome(row: HomeRowAt): Promise<AccountsActed<SharedHomeRead>> {
    return await this.act<SharedHomeRead>("POST", "/shared_home/add", row);
  }

  async bringFromSharedHome(row: HomeRowAt): Promise<AccountsActed<SharedHomeRead>> {
    return await this.act<SharedHomeRead>("POST", "/shared_home/bring", row);
  }

  async chooseSharedHomeSide(row: HomeRowAt, side: HomeSide): Promise<AccountsActed<SharedHomeRead>> {
    return await this.act<SharedHomeRead>("POST", "/shared_home/choose", { row, side });
  }

  async homeKinds(login: string): Promise<AccountsActed<HomeKinds>> {
    return await this.act<HomeKinds>("GET", `/shared_home/kinds?login=${encodeURIComponent(login)}`);
  }

  async setHomeKind(login: string, kind: HomeKind, on: boolean): Promise<AccountsActed<HomeKinds>> {
    return await this.act<HomeKinds>("POST", "/shared_home/kinds/set", { login, kind, on });
  }

  async setHomeItem(row: HomeRowAt, on: boolean): Promise<AccountsActed<HomeKinds>> {
    return await this.act<HomeKinds>("POST", "/shared_home/items/set", { ...row, on });
  }

  async setHomeRepository(login: string, owner: string, name: string): Promise<AccountsActed<SharedHomeRead>> {
    return await this.act<SharedHomeRead>("POST", "/shared_home/repository", { login, owner, name });
  }

  private async act<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<AccountsActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as T };
  }
}
