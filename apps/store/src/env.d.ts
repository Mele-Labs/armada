import type { Account } from "./account";

declare global {
  namespace Cloudflare {
    interface Env {
      ACCOUNT: DurableObjectNamespace<Account>;
      ENROL_LIMIT: RateLimit;
      FORGE_USER_URL?: string;
    }
  }
  interface Env extends Cloudflare.Env {}
}
