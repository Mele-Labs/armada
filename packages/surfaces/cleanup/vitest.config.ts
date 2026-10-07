// Cleanup's tests: pure modules in node, and its screens mounted in Chromium. `.test.ts` is node and
// `.test.tsx` the browser. The browser settings are `@armada/desktop/vitest-preset`'s, shared by every surface.
import { browserProject } from "@armada/desktop/vitest-preset";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: "cleanup",
          environment: "node",
          include: ["src/**/*.test.ts"],
        },
      },
      browserProject("cleanup (browser)", ["src/**/*.test.tsx"]),
    ],
  },
});
