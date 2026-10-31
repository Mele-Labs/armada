// The repository picker with my-store picked: Cockpit bands its Jobs under store-web and store-api, Studios draws a panel for each, and scratch, store-mobile and the
// platform repositories are not drawn. Six repositories are served, as in `workspaces-and-autonomy`, on All repositories. The walk `workspace-scope` plays it.

import type { Studio } from "@armada/protocol";

import { keeping } from "../studios-fake";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { REPOSITORIES, workspacesAndAutonomy } from "./workspaces-and-autonomy";

const now = () => new Date().toISOString();
const working = { turn: { state: "working" as const }, mode: "auto" as const, running: true };

const job = (id: string, name: string, title: string) => {
  const repo = REPOSITORIES.find((one) => one.manifest?.repository === name)!;
  return hosted(id, { manifest_id: repo.manifest!.id, cwd: repo.root, title, last_turn_at: now(), hosted: working });
};

const records = () => [
  job("01SCOPEWEBAAAAAAAAAAAAAAAA", "store-web", "Round the cart totals"),
  job("01SCOPEWEBBBBBBBBBBBBBBBBB", "store-web", "Lay out the gift card page"),
  job("01SCOPEAPIAAAAAAAAAAAAAAAA", "store-api", "Add the gift card endpoint"),
  job("01SCOPEDOCSAAAAAAAAAAAAAAA", "store-docs", "Write the release notes"),
  job("01SCOPESCRATCHAAAAAAAAAAAA", "scratch", "Try the new parser"),
  job("01SCOPEMOBILEAAAAAAAAAAAAA", "store-mobile", "Fix the tab bar"),
];

let minted = 0;
const studio = (name: string, repo: string): Studio => {
  const at = "2026-10-29T09:00:00Z";
  minted += 1;
  return { id: `01SCOPESTUDIO${String(minted).padStart(13, "0")}`, manifest_id: REPOSITORIES.find((one) => one.manifest?.repository === repo)!.manifest!.id, name, created_at: at, touched_at: at, nodes: [], edges: [] };
};

const studios = keeping([
  studio("Gift card flow", "store-web"),
  studio("Checkout states", "store-web"),
  studio("Gift card endpoint", "store-api"),
  studio("Release notes layout", "store-docs"),
  studio("Parser sketches", "scratch"),
]);

const sessions = new FakeSessionsFleet(records(), {}).scenario({ ...workspacesAndAutonomy, state: { ...workspacesAndAutonomy.state, repository: null }, draft: { workspace: workspacesAndAutonomy.draft!.workspace } });

export const workspaceScope: Scenario = {
  ...sessions,
  name: "workspace-scope",
  says: "Six repositories on All repositories; my-store holds store-web and store-api. Picking it narrows Cockpit and Studios to those two",
  behaves: (fleet) => ({ ...sessions.behaves!(fleet), ...studios.routes(fleet as never) }),
};
