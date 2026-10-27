// Mods that travel: a work machine with the personal login added, against that login's shared home. Theme lists
// Dusk, the Harbour Armada ships beside a local Harbour, and Ember, which only the home has; Promote on Dusk opens a
// pull request and the Job that holds it. Kit lists the same mods with the same marks, and What syncs on the added
// profile has a Mods switch, off until it is turned on. Built over `shared-home`; the walk `mods-travel` plays it.

import type { PullRequestPage } from "@armada/protocol";
import type { SharedRow } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import type { PanelReads, PullRequestRead } from "../../shared-panel/reads";
import { sharedPanelReads } from "../shared-panel-reads";
import { FakeSessionsFleet, held, hosted, pullRequest } from "../sessions-fleet";
import { DUSK_CSS, promoteThemesWith } from "../themes";
import { sharedHome } from "./shared-home";

const ARMADA = repository();
const MANIFEST = ARMADA.manifest!.id;
const PULL = 2841;
const ADDRESS = `https://forge.example/mele-labs/armada/pull/${PULL}`;
const PULL_TITLE = "Add the Dusk theme";
const JOB = "01SESSIONPROMOTEDUSKAAAAAA";
const JOB_TITLE = "Promote Dusk";
const BRANCH = "armada/mod-dusk-1761226080000";

/** The home's Dusk: the other machine warmed the accent. */
const HOME_DUSK = DUSK_CSS.replace("--accent: #C1B4EC;", "--accent: #D9896B;").replace("--accent-hover: #D0C5F3;", "--accent-hover: #E39C82;");

const mod = (name: string, state: SharedRow["state"], modKind: "theme" | "layout", more: Partial<SharedRow> = {}): SharedRow => ({ kind: "mods", name, state, modKind, ...more });

const MOD_ROWS: SharedRow[] = [
  mod("dusk", "differs", "theme", { here: DUSK_CSS, there: HOME_DUSK }),
  mod("harbour", "here", "theme"),
  mod("ember", "home", "theme"),
  mod("tidy", "same", "layout"),
];

const minutesAgo = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

/** The pull request Promote opened, as the forge reads it: the mod's two files under `packages/mods/dusk/`. */
function promoted(): PullRequestRead {
  const page: PullRequestPage = {
    manifest_id: MANIFEST,
    number: PULL,
    base: "main",
    author: "claude",
    created_at: minutesAgo(1),
    additions: DUSK_CSS.split("\n").length + 3,
    deletions: 0,
    mergeable: "mergeable",
    review: "none",
    approvals: 0,
    body: "Copies the Dusk theme into packages/mods/dusk.",
    conversation: [],
    checks: [{ name: "build", state: "running", address: `${ADDRESS}/checks?check_run_id=1` }],
    files: [
      { path: "packages/mods/dusk/mod.toml", status: "added", additions: 3, deletions: 0 },
      { path: "packages/mods/dusk/theme.css", status: "added", additions: DUSK_CSS.split("\n").length, deletions: 0 },
    ],
    commits: [{ sha: "d5c1a0e", headline: PULL_TITLE, author: "claude", at: minutesAgo(1) }],
  };
  return {
    repository: "mele-labs/armada",
    root: ARMADA.root,
    facts: { kind: "pull_request", number: PULL, title: PULL_TITLE, branch: BRANCH, address: ADDRESS, checks: { state: "pending" }, state: "open", auto: false, body: page.body ?? "" },
    page,
  };
}

const MOD_TOML = 'name = "dusk"\nkind = "theme"\nversion = "1"';
const added = (text: string) => `@@ -0,0 +1,${text.split("\n").length} @@\n${text.split("\n").map((line) => `+${line}`).join("\n")}`;

function reads(): PanelReads {
  const base = sharedPanelReads({ lines: [] });
  return {
    ...base,
    pullRequest: (item) => (item.number === PULL ? Promise.resolve(promoted()) : base.pullRequest(item)),
    pullRequestPatches: (item) =>
      item.number === PULL
        ? Promise.resolve({
            ok: true,
            files: [
              { manifest_id: MANIFEST, number: PULL, path: "packages/mods/dusk/mod.toml", patch: added(MOD_TOML) },
              { manifest_id: MANIFEST, number: PULL, path: "packages/mods/dusk/theme.css", patch: added(DUSK_CSS) },
            ],
          })
        : (base.pullRequestPatches?.(item) ?? Promise.resolve({ ok: false, said: "The forge could not find this pull request." })),
  };
}

function build(): Scenario {
  const home = sharedHome.draft!.sharedHome!;
  const base: Scenario = {
    ...sharedHome,
    draft: {
      ...sharedHome.draft,
      sharedHome: { ...home, rows: [...home.rows, ...MOD_ROWS] },
      sharedPanel: reads,
      account: {
        signIn: { login: "mele-work", code: "RTHV-2CLM", repository: home.repository },
        add: {
          login: home.account,
          code: "WDJB-MJHT",
          items: {
            mods: [
              { name: "dusk", scope: "global", repo: "" },
              { name: "harbour", scope: "private", repo: "" },
              { name: "tidy", scope: "global", repo: "" },
            ],
          },
        },
        joined: [
          { login: "mele-work", chosen: false },
          { login: home.account, chosen: true },
        ],
      },
    },
  };
  const served = new FakeSessionsFleet([]).scenario(base);
  return {
    ...served,
    name: "mods-travel",
    says: "Mods against the shared home: Promote opens a pull request and its Job, a mod Armada ships, Kit's Mods and the Mods switch in What syncs",
    behaves: (handle) => {
      const kit = sharedHome.behaves?.(handle) ?? {};
      const fake = new FakeSessionsFleet([]);
      const own = fake.scenario(base).behaves!(handle);
      // Promote copies the mod onto a branch, opens the pull request and starts the Job that holds it.
      promoteThemesWith((name) => {
        fake.changed(
          hosted(JOB, {
            title: JOB_TITLE,
            manifest_id: MANIFEST,
            cwd: ARMADA.root,
            started_at: new Date().toISOString(),
            attachments: [held("slot", "4"), pullRequest(PULL, { title: PULL_TITLE, branch: BRANCH, address: ADDRESS, checks: "pending" })],
          }),
        );
        return { branch: `armada/mod-${name}-1761226080000`, pull: { number: PULL, title: PULL_TITLE, address: ADDRESS }, job: { id: JOB, title: JOB_TITLE } };
      });
      return { ...kit, ...own };
    },
  };
}

export const modsTravel: Scenario = build();
