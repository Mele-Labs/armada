import type { Account } from "./account";

declare global {
  namespace Cloudflare {
    interface Env {
      ACCOUNT: DurableObjectNamespace<Account>;
      /** Absent when Cloudflare refused the binding (it may not be on the Free plan): enrol then goes on uncounted. */
      ENROL_LIMIT?: RateLimit;
      FORGE_USER_URL?: string;
      /** Makes this a team store: `org` or `org/team-slug`. Missing or empty is a personal store. */
      TEAM?: string;
    }
  }
  interface Env extends Cloudflare.Env {}
}
