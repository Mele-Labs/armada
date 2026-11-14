import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

// `FORGE_USER_URL` points enrol, and `FORGE_LOGIN_URL` the phone's sign-in, at names the tests answer themselves.
// A rate-limit test signs and sends over a hundred requests, which a busy CI runner took past the 5s default.
export default defineConfig({
  test: {
    testTimeout: 30_000,
  },
  plugins: [
    cloudflareTest({
      wrangler: {
        configPath: "./wrangler.jsonc",
      },
      miniflare: {
        bindings: {
          FORGE_USER_URL: "https://forge.test/user",
          FORGE_LOGIN_URL: "https://forge.test",
        },
      },
    }),
  ],
});
