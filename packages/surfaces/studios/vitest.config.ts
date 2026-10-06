// Studios' tests: pure modules in node, and the whole app on the mock Fleet in
// Chromium. `.test.ts` is node and `.test.tsx` the browser. The browser
// settings are `@armada/desktop/vitest-preset`'s, shared by every surface.
import { browserProject } from "@armada/desktop/vitest-preset";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "studios",
          environment: "node",
          include: ["src/**/*.test.ts"],
        },
      },
      browserProject("studios (browser)", ["src/**/*.test.tsx"]),
    ],
  },
});
