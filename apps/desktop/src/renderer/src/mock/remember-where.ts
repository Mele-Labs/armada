// Two repositories whose runs Fleet saw, for the walk `remember-where`: Remember asks where at a repository's first, and keeps in the same place after.
// Mock only. A later Job on `my-app` runs `pnpm lint`, which is seen and not yet kept, so the next Remember has something to keep.

import { keptRuns, type RunsRead } from "./runs-kept";
import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

export const MY_APP = "/Users/user/code/my-app";
export const API = "/Users/user/code/api";

const TESTS = { id: "tests", job: "Tests", command: "pnpm test" };
const DEV = { id: "dev", job: "Dev server", command: "pnpm dev", port: 5173 };
const LINT = { id: "lint", job: "Lint", command: "pnpm lint" };

export function rememberWhereStore(): SessionsStore {
  const inner = sessionsStore([], [], new Map(), []);
  let linted = false;
  const bump = () => inner.edit("none", (one) => one);
  const read = (root: string): RunsRead | undefined => {
    if (root === MY_APP) return { repository: "my-app", source: "From the last Job", jobs: linted ? [TESTS, DEV, LINT] : [TESTS, DEV] };
    if (root === API) return { repository: "api", source: "From the last Job", jobs: [TESTS] };
    return undefined;
  };
  return {
    ...inner,
    later() {
      linted = true;
      bump();
    },
    runs: keptRuns(read, bump),
  };
}
