// Dispatch's members: staged pictures, `@` paths and the repository a request is about.

import type { DispatchApi } from "../../../../shared/api/dispatch";
import type { Slice } from "../fake-context";
import { READ_MS, requestRepositoryRead } from "../request-repository-fleet";

/** The paths the mock's `@` can find, narrowed by what follows it as Fleet's search narrows them. */
const FILES = [
  "README.md",
  "crates/fleet/src/sessions.rs",
  "docs/practices/bridge.md",
  "docs/practices/session-composer.md",
  "packages/components/src/compositions/SessionComposer/SessionComposer.tsx",
  "packages/components/src/compositions/SessionComposer/InlineTag.tsx",
  "apps/desktop/src/renderer/src/sessions.tsx",
  "apps/desktop/src/renderer/src/mock/sessions/script.ts",
];

export const dispatch: Slice<DispatchApi, Record<string, never>> = {
  name: "dispatch",
  state: {},
  api: (_scenario, { state }) => ({
    stageAttachment: async (_bytes, filename) => ({ path: filename }),
    searchFiles: async (query) => FILES.filter((path) => path.toLowerCase().includes(query.toLowerCase())),
    readRequestRepository: async (words) => {
      await new Promise((resume) => setTimeout(resume, READ_MS));
      return requestRepositoryRead(words, state().holds.repositories ?? []);
    },
  }),
};
