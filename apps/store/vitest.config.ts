import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

// `FORGE_USER_URL` points enrol at a name the tests answer themselves.
export default defineConfig({
  plugins: [
    cloudflareTest({
      wrangler: { configPath: "./wrangler.jsonc" },
      miniflare: { bindings: { FORGE_USER_URL: "https://forge.test/user" } },
    }),
  ],
});
