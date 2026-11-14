import type { Account } from "./account";

declare global {
  namespace Cloudflare {
    interface Env {
      ACCOUNT: DurableObjectNamespace<Account>;
      ENROL_LIMIT: RateLimit;
      FORGE_USER_URL?: string;
      /** Makes this a team store: `org` or `org/team-slug`. Missing or empty is a personal store. */
      TEAM?: string;
    }
  }
  interface Env extends Cloudflare.Env {}
}
