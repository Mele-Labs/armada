// The palette read like Raycast's: Jobs in two repositories at work, waiting on him and paused, open pull requests,
// issues, tasks and the rows a Commands mod brings. The ship at rest with the review surface behind it, so a pull request
// opened from the palette lands in Reviews. Played by the walk of the same name.

import type { Projects } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { PaletteFixtures, PaletteLinked } from "../../palette-fixtures";
import type { Scenario } from "../moment";
import { s262PrReview as base } from "./pr-review";

const ARMADA = repository().manifest!.id;
const POCKET = "pocket";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** Parked by him in the other repository, beside the one at work in armada that shares its first words. */
const PAUSED: Session = {
  id: "pk1",
  title: "Migration notes for Pocket",
  manifestId: POCKET,
  turn: { state: "idle" },
  lastTurnAt: ago(30),
  standing: "idle",
  paused: true,
  rows: [
    { id: "pk1-u", at: ago(60), kind: "message", from: { kind: "you" }, text: "Write the migration notes for the install prompt." },
    { id: "pk1-a", at: ago(30), kind: "message", from: { kind: "agent" }, text: "Two notes are drafted. The icon sizes are left." },
  ],
  attachments: [],
};

const FORGE = "https://forge.example/mele-labs";
const pull = (repo: string, number: number, title: string): PaletteLinked => ({ repo, number, title, url: `${FORGE}/${repo}/pull/${number}` });
const issue = (repo: string, number: number, title: string): PaletteLinked => ({ repo, number, title, url: `${FORGE}/${repo}/issues/${number}` });

const PALETTE: PaletteFixtures = {
  pulls: [
    pull("armada", 1899, "Cache the plan windows for a minute"),
    pull("armada", 1903, "Serve a bay's history from one query"),
    pull("armada", 1896, "Fix the reconnect wait"),
    pull("armada", 1892, "Add the pause marker column"),
    pull("armada", 1890, "Gate policy on every run"),
    pull("pocket", 212, "Bump the manifest icons"),
  ],
  issues: [issue("armada", 2214, "Plan windows refresh twice a minute"), issue("armada", 2230, "A Job is not found by its pull request"), issue("pocket", 88, "The install prompt shows the old icon")],
  commands: [
    { id: "security-review", label: "Review a pull request for security", kind: "prompt", value: "{pr}", alias: "sec", hotkey: "⇧⌘S" },
    { id: "grafana", label: "Open Grafana for this repo", kind: "url", value: "grafana.example.com", alias: "graf", hotkey: "⌘G" },
    { id: "audit", label: "Run pnpm audit", kind: "shell", value: "pnpm audit", alias: "audit" },
    { id: "triage", label: "Triage: Projects then Add a task", kind: "chain", alias: "triage" },
  ],
  pinned: ["session:q1", "cmd:grafana"],
  recent: ["pull:armada:1896", "nav:reviews", "cmd:audit", "session:w1"],
};

/** What Fleet lists for the Palette and Offline install projects, so a task can be filed in either and the palette finds it by title. */
const task = (id: string, manifest_id: string, title: string) => ({ kind: "task" as const, id, manifest_id, title, notes: "", state: "open" as const });
const PROJECTS: Projects = {
  projects: [
    { id: "palette", name: "Palette", rows: [task("task-columns", ARMADA, "Columns for the palette rows"), task("task-tick", ARMADA, "Tick a task from the palette")] },
    { id: "offline-install", name: "Offline install", rows: [task("task-icons", POCKET, "Warm the install icons")] },
  ],
  loose: [],
};

const sessions = base.draft!.sessions!;

export const s332PaletteLikeRaycast: Scenario = {
  ...base,
  name: "palette-like-raycast",
  says: "The palette with Jobs in two repositories, pull requests, issues, tasks and mod commands: pinned and recent when empty, a panel of acts on ⌘K",
  draft: {
    ...base.draft,
    palette: PALETTE,
    projectsTab: PROJECTS,
    // The ship's store has no pause, which Fleet answers: parking a Job rests it and Resume sets it going again.
    sessions: (control, more = []) => {
      const store = sessions(control, [...more, PAUSED]);
      return {
        ...store,
        pause: async (id) => (store.edit(id, (one) => ({ ...one, paused: true, turn: { state: "idle" }, standing: "idle" })), true),
        resume: async (id) => {
          store.edit(id, ({ paused: _paused, ...one }) => ({ ...one, turn: { state: "working" }, standing: "working" }));
          return true;
        },
      };
    },
  },
};
