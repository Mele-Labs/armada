// Starting out: a new project, the folders a person already works in with another agent CLI, how a repository
// runs and where that is kept, and the mod's install. A slice imports protocol, never another slice;
// `../api.ts` and `../bridge.ts` compose them. `docs/practices/protocol.md`, *Onboarding*.

import type { LocateAnswer } from "@armada/screens/src/locate-reads";
import type {
  DropRunFact,
  ImportFolders,
  ImportRuns,
  KeptOnThisMac,
  ModInstalled,
  Outcome,
  RunMemory,
  RunMemoryPlace,
  SetRunMemoryPlace,
} from "@armada/protocol";

/** What one of these reads or acts came to: Fleet's answer, or the refusal to word. */
export type OnboardingAnswer<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type OnboardingApi = {
  /** `POST /repositories/create`: make `name` under `parent`, start it as a git repository and serve it. Listed before this answers; the window picks it. */
  createProject: (parent: string, name: string) => Promise<LocateAnswer>;
  /** `GET /import/folders`: the folders the person's agent CLI has kept sessions for, newest first. */
  listImportFolders: () => Promise<OnboardingAnswer<ImportFolders>>;
  /** `GET /import/runs?path=`: how one of those folders was run, read from its past sessions. */
  getImportRuns: (path: string) => Promise<OnboardingAnswer<ImportRuns>>;
  /** `POST /import/install_mod`: install Armada's mod into the agent CLI. **Send it only for a press**; Fleet refuses `requested: false`. */
  installAgentMod: () => Promise<OnboardingAnswer<ModInstalled>>;
  /** `GET /repositories/runs?repository=`: how a served repository runs, each fact with where it is kept. */
  getRunMemory: (repository: string) => Promise<OnboardingAnswer<RunMemory>>;
  /** `GET /repositories/runs/place?repository=`: where its remembered runs are kept. */
  getRunMemoryPlace: (repository: string) => Promise<OnboardingAnswer<RunMemoryPlace>>;
  /** `POST /repositories/runs/place`: choose where they are kept. `cloud` and a bad `synced_folder` are refused. */
  setRunMemoryPlace: (body: SetRunMemoryPlace) => Promise<OnboardingAnswer<RunMemoryPlace>>;
  /** `GET /repositories/kept_on_this_mac?repository=`: whether every piece and run fact for it stays on this Mac. */
  getKeptOnThisMac: (repository: string) => Promise<OnboardingAnswer<KeptOnThisMac>>;
  /** `POST /repositories/kept_on_this_mac`: mark it or not. Marking moves its run memory to This Mac. */
  setKeptOnThisMac: (body: KeptOnThisMac) => Promise<OnboardingAnswer<KeptOnThisMac>>;
  /** `POST /repositories/runs/drop`: never offer or write this line again. Answers the memory as it now stands. */
  dropRunFact: (body: DropRunFact) => Promise<OnboardingAnswer<RunMemory>>;
};

export const ONBOARDING_CHANNELS = {
  createProject: "bridge:create-project",
  listImportFolders: "bridge:list-import-folders",
  getImportRuns: "bridge:get-import-runs",
  installAgentMod: "bridge:install-agent-mod",
  getRunMemory: "bridge:get-run-memory",
  getRunMemoryPlace: "bridge:get-run-memory-place",
  setRunMemoryPlace: "bridge:set-run-memory-place",
  dropRunFact: "bridge:drop-run-fact",
  getKeptOnThisMac: "bridge:get-kept-on-this-mac",
  setKeptOnThisMac: "bridge:set-kept-on-this-mac",
} as const;
