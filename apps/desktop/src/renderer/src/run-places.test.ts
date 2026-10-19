import { expect, test } from "vitest";
import type { RunMemory } from "@armada/protocol";

import { boundProfile, defaultParent, placeOf, rootOfImport, runsOf, wireOfPlace } from "./run-places";

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

test("cloud storage is asked for as a bucket where an address was typed, and as the store where a profile row was chosen", () => {
  expect(wireOfPlace("/r", { id: "cloud", detail: " s3://armada/runs " }, [])).toEqual({ repository: "/r", place: "cloud", cloud: { bucket: "s3://armada/runs" } });
  expect(wireOfPlace("/r", { id: "cloud", detail: "gs://b/p" }, [])).toEqual({ repository: "/r", place: "cloud", cloud: { bucket: "gs://b/p" } });
  expect(wireOfPlace("/r", { id: "cloud", detail: "Armada's store, nickmele" }, [])).toEqual({ repository: "/r", place: "cloud", cloud: { store: {} } });
  // Something that is neither is left for Fleet to refuse in its own words.
  expect(wireOfPlace("/r", { id: "cloud", detail: "my bucket" }, [])).toEqual({ repository: "/r", place: "cloud" });
});

test("a cloud place is drawn as its bucket address, or as the store with the profile it is under where one is known", () => {
  expect(placeOf({ place: "cloud", cloud: { bucket: "r2://armada/runs" } }, [])).toEqual({ id: "cloud", detail: "r2://armada/runs" });
  expect(placeOf({ place: "cloud", cloud: { store: {} } }, [], "nickmele")).toEqual({ id: "cloud", detail: "Armada's store, nickmele" });
  expect(placeOf({ place: "cloud", cloud: { store: {} } }, [])).toEqual({ id: "cloud", detail: "Armada's store" });
  expect(placeOf({ place: "cloud" }, [])).toEqual({ id: "cloud" });
});

test("the profile a repository is bound to is the one whose list of repositories holds its root", () => {
  const profile = (login: string, roots: string[]) => ({ account_id: 1, login, active: false, chosen: false, repositories: roots.map((root) => ({ root, name: root, by: "matched" as const })) });
  const account = { profiles: [profile("a", ["/x"]), profile("b", ["/r"])] };
  expect(boundProfile(account, "/r")).toBe("b");
  expect(boundProfile(account, "/none")).toBeUndefined();
  expect(boundProfile(undefined, "/r")).toBeUndefined();
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
