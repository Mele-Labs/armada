// Set up with a job: what the Session's first message says, and what a refusal at either
// call comes back as.

import { describe, expect, it } from "vitest";

import type { ManifestProposal, Outcome, ScannedWorkspace } from "@armada/protocol";
import { readInto, type SetupOpen } from "./setup-view";
import { setupMessageOf, setupTitleOf, startSetupSession, type SetupSessionFleet } from "./setup-session";

function workspace(dir: string, evidence: string, files: string[]): ScannedWorkspace {
  return {
    dir, declared_by: [], evidence, manifests: files.map((file) => ({ file, tool: "npm" })), lockfiles: [],
    runnables: [], tools: [], services: [], ports: [], missing: [], not_read: [],
  };
}

function proposal(dir: string, checks: string[]): ManifestProposal {
  return {
    dir,
    file: `${dir}/armada.yml`,
    id: { value: dir, provenance: { source: "convention" } },
    ports: [],
    checks: checks.map((name) => ({ name, run: `yarn ${name}`, provenance: { source: "read", file: `${dir}/package.json`, key: `scripts.${name}` } })),
    commands: [],
    policy: [],
  };
}

const HELD = readInto(
  { state: "reading" },
  {
    checkout: "/repo/monolith",
    workspaces: [
      workspace("packages/web", "strong", ["packages/web/package.json"]),
      workspace("gems/billing", "thin", ["gems/billing/Gemfile"]),
      workspace("test/bats", "not_followed", []),
    ],
    not_read: [],
  },
  {
    checkout: "/repo/monolith",
    proposals: [proposal("packages/web", ["test", "lint"]), proposal("gems/billing", []), proposal("test/bats", [])],
    caps: { cost_micros: 5_000_000, turns: 200 },
  },
) as SetupOpen;

describe("the first message", () => {
  const message = setupMessageOf(HELD);

  it("names onboard-repo and states its rules, so a Session without the skill keeps them", () => {
    expect(message).toContain("onboard-repo");
    expect(message).toContain("one question at a time");
    expect(message).toContain("Write nothing before I confirm it");
    expect(message).toContain("`armada check <name>`");
  });

  it("asks before writing whether to ignore armada.yml, saying what ignoring it costs Jobs", () => {
    const asks = message.split("\n").find((line) => line.includes(".gitignore")) ?? "";
    expect(asks).toContain("Before writing it, ask me whether to add armada.yml to this repository's .gitignore");
    expect(asks).toContain("do what I answer");
    expect(asks).toContain("missing from the worktrees Jobs work in");
    expect(message.indexOf(".gitignore")).toBeLessThan(message.indexOf("End on a real run"));
  });

  it("says which files Fleet lets it write, a workspace's own armada.yml among them", () => {
    expect(message).toContain("A workspace that needs its own writes its armada.yml in its own directory.");
    expect(message).toContain("Write only armada.yml files, .gitignore files and .git/info/exclude");
  });

  it("carries each workspace's reading on a line of its own", () => {
    expect(message).toContain(
      "- `packages/web` (read packages/web/package.json): ready to write. Checks: test `yarn test` (packages/web/package.json scripts.test, read); lint `yarn lint` (packages/web/package.json scripts.lint, read).",
    );
    expect(message).toContain(
      "- `gems/billing` (read gems/billing/Gemfile): no checks proposed. No file here names a script, so anything proposed is convention.",
    );
    expect(message).toContain("- `test/bats` (no files read): no checks proposed. Nothing here could be read.");
  });
});

it("titles the Session by the repository's folder", () => {
  expect(setupTitleOf("/repo/monolith")).toBe("Set up monolith");
});

const REFUSED: Outcome = { ok: false, why: "refused", error: { code: "fleet.refused", message: "", run_id: "", fields: {}, chain: [] } };

describe("starting it", () => {
  it("starts in the repository being set up and sends the message to that Session", async () => {
    const calls: unknown[] = [];
    const fleet: SetupSessionFleet = {
      startSession: async (title, root) => (calls.push({ title, root }), { ok: true, value: { id: "01S" } }),
      sendSessionMessage: async (send) => (calls.push(send), { ok: true }),
    };
    expect(await startSetupSession(HELD, "/repo/monolith", fleet, () => "")).toEqual({ ok: true, id: "01S" });
    expect(calls).toEqual([{ title: "Set up monolith", root: "/repo/monolith" }, { session_id: "01S", text: setupMessageOf(HELD) }]);
  });

  it("says a refused start in words, and a refused send with the Session it started", async () => {
    const refusing: SetupSessionFleet = {
      startSession: async () => ({ ok: false, outcome: REFUSED }),
      sendSessionMessage: async () => ({ ok: true }),
    };
    expect(await startSetupSession(HELD, "/r", refusing, () => "")).toEqual({ ok: false, saying: "Job not started: Fleet refused it" });
    const deaf: SetupSessionFleet = {
      startSession: async () => ({ ok: true, value: { id: "01S" } }),
      sendSessionMessage: async () => ({ ok: false, outcome: REFUSED }),
    };
    expect(await startSetupSession(HELD, "/r", deaf, () => "it ended")).toEqual({
      ok: false,
      id: "01S",
      saying: "Job started, but the reading was not sent: it ended",
    });
  });
});
