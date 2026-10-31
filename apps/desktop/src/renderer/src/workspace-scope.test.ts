// The Workspace the window is scoped to, held for the Sessions store.

import { afterEach, expect, test, vi } from "vitest";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onWorkspaceScope, scopeToWorkspace, workspaceScope } from "./workspace-scope";

afterEach(() => scopeToWorkspace(null));

test("a Workspace is held until none is, and each change is heard", () => {
  const heard = vi.fn();
  const off = onWorkspaceScope(heard);
  scopeToWorkspace([repository()]);
  expect(workspaceScope()).toEqual([repository()]);
  scopeToWorkspace(null);
  expect(workspaceScope()).toBeNull();
  expect(heard).toHaveBeenCalledTimes(2);
  off();
});

test("the same repositories again is not a change", () => {
  scopeToWorkspace([repository()]);
  const heard = vi.fn();
  const off = onWorkspaceScope(heard);
  scopeToWorkspace([repository()]);
  expect(heard).not.toHaveBeenCalled();
  off();
});
