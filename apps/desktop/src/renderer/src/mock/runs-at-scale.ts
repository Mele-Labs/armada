// A repository with workspaces, whose runs Fleet saw across them, for the walk `runs-at-scale`: most are kept already, two are new since the last
// Remember. Dispatch shows only those two, each with the workspace it ran in, and all of it is one press away. Mock only.

import { keptRuns, type RunsRead } from "./runs-kept";
import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

export const MY_APP = "/Users/user/code/my-app";

const WEB = "apps/web";
const API = "packages/api";
const UI = "packages/ui";

const JOBS: RunsRead["jobs"] = [
  { id: "root-tests", job: "Tests", command: "pnpm test" },
  { id: "root-lint", job: "Lint", command: "pnpm lint" },
  { id: "web-dev", job: "Dev server", workspace: WEB, command: "pnpm dev", port: 5173 },
  { id: "web-tests", job: "Tests", workspace: WEB, command: "pnpm test" },
  { id: "web-build", job: "Build", workspace: WEB, command: "pnpm build" },
  { id: "web-lint", job: "Lint", workspace: WEB, command: "pnpm lint" },
  { id: "api-dev", job: "Dev server", workspace: API, command: "pnpm dev", port: 3000 },
  { id: "api-tests", job: "Tests", workspace: API, command: "pnpm test" },
  { id: "api-build", job: "Build", workspace: API, command: "pnpm build" },
  { id: "ui-tests", job: "Tests", workspace: UI, command: "pnpm test" },
  { id: "ui-storybook", job: "Storybook", workspace: UI, command: "pnpm storybook", port: 6006 },
  { id: "ui-build", job: "Build", workspace: UI, command: "pnpm build" },
];

/** The two a later Job ran that the last Remember did not have. */
const NEW = new Set(["web-lint", "ui-build"]);

export function runsAtScaleStore(): SessionsStore {
  const inner = sessionsStore([], [], new Map(), []);
  const bump = () => inner.edit("none", (one) => one);
  const read = (root: string): RunsRead | undefined => (root === MY_APP ? { repository: "my-app", source: "From the last Job", jobs: JOBS } : undefined);
  return {
    ...inner,
    runs: keptRuns(read, bump, { root: MY_APP, kept: JOBS.filter((one) => !NEW.has(one.id)).map((one) => one.id), place: { id: "mac" } }),
  };
}
