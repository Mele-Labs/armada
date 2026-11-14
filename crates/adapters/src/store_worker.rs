//! The claims store's Worker as Fleet deploys it, bundled from `apps/store` by `pnpm --dir apps/store bundle`,
//! and the Cloudflare page that makes a token able to deploy it. `apps/store:bundle_current` refuses a
//! bundle older than its source.

/// The bundled Worker module.
pub const STORE_WORKER: &str = include_str!("../../../apps/store/dist/worker.js");

/// Cloudflare's page that makes an API token, filled in with what a store's deploy needs: Workers
/// Scripts edit and Account Settings read, under the name the token is listed as.
pub const CLOUDFLARE_TOKEN_PAGE: &str = "https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=%5B%7B%22key%22%3A%22workers_scripts%22%2C%22type%22%3A%22edit%22%7D%2C%7B%22key%22%3A%22account_settings%22%2C%22type%22%3A%22read%22%7D%5D&name=Armada%20claims%20store";
