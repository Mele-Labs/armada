import { expect, test } from "vitest";
import type { RunMemory } from "@armada/protocol";

import { defaultParent, placeOf, rootOfImport, runsOf, wireOfPlace } from "./run-places";

test("a new project is made in the folder under home that most past sessions share", () => {
  const paths = [
    "/Users/nick/Development/armada",
    "/Users/nick/Development/armada/.armada/slots/slot-2",
    "/Users/nick/Sites/blog",
    "/Users/nick/Development/api",
  ];
  expect(defaultParent(paths)).toBe("/Users/nick/Development");
});

test("a tie goes to the folder of the newest session", () => {
  expect(defaultParent(["/Users/nick/Sites/blog", "/Users/nick/Development/api"])).toBe("/Users/nick/Sites");
});

test("a project directly under home says nothing of where projects are kept, so ~/code is the answer", () => {
  expect(defaultParent(["/Users/nick/notes", "/Users/nick/scratch"])).toBe("~/code");
  expect(defaultParent([])).toBe("~/code");
});

test("a folder inside a repository is added as that repository", () => {
  const git = { state: "inside", root: "/Users/nick/code/shop" } as const;
  expect(rootOfImport({ path: "/Users/nick/code/shop/web", exists: true, last_session_at: "", sessions: 1, git })).toBe("/Users/nick/code/shop");
  expect(rootOfImport({ path: "/Users/nick/code/api", exists: true, last_session_at: "", sessions: 1, git: { state: "root" } })).toBe("/Users/nick/code/api");
});

test("a synced place is asked for with the folder it was chosen as", () => {
  expect(wireOfPlace("/r", { id: "mac" }, [])).toEqual({ repository: "/r", place: "this_mac" });
  expect(wireOfPlace("/r", { id: "synced", detail: "/Users/nick/Drive" }, [])).toEqual({ repository: "/r", place: "synced", synced_folder: "/Users/nick/Drive" });
  expect(wireOfPlace("/r", { id: "synced", detail: "iCloud Drive" }, [{ label: "iCloud Drive", path: "/Users/nick/iCloud" }])).toEqual({ repository: "/r", place: "synced", synced_folder: "/Users/nick/iCloud" });
  expect(placeOf({ place: "synced", synced_folder: "/Users/nick/Drive" }, [])).toEqual({ id: "synced", detail: "~/Drive" });
});

const MEMORY: RunMemory = {
  repository: "/r",
  place: "repository",
  workspaces: [
    {
      workspace: ".",
      runs: [{ workspace: ".", kind: "tests", command: "pnpm test", succeeded: true, last_ran: "", source: "claude_code_sessions", state: "waiting", why: "No Job branch to write it on yet" }],
    },
    {
      workspace: "web",
      runs: [{ workspace: "web", kind: "dev_server", command: "pnpm dev", port: 5173, succeeded: true, last_ran: "", source: "jobs", state: "kept", where: "repository" }],
    },
  ],
};

test("a line the place has not taken carries what it waits on, and a root line has no workspace", () => {
  const { rows, source } = runsOf("my-app", MEMORY, []);
  expect(rows).toEqual([
    { id: expect.any(String), job: "Tests", command: "pnpm test", waiting: "No Job branch to write it on yet" },
    { id: expect.any(String), job: "Dev server", workspace: "web", command: "pnpm dev", port: 5173 },
  ]);
  expect(source).toBe("From Claude Code sessions, Jobs");
});
