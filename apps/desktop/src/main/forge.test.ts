// What main will and will not hand to the OS when a person clicks a Studio node.
//
// **The address is the whole subject.** The string reaching `openExternal` is the one the Studio
// recorded and never one the renderer composed, and a record carrying something other than a web
// address is refused by name rather than opened.
//
// **Electron is mocked, and only `shell.openExternal`.** What is asserted here is the decision in
// front of the call, so the call is a spy and the shell is not under test.

import { beforeEach, describe, expect, it, vi } from "vitest";

const openExternal = vi.fn(async (_url: string) => undefined);
vi.mock("electron", () => ({ shell: { openExternal: (url: string) => openExternal(url) } }));

const { openStudioNode } = await import("./forge");

import { NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { Studio, StudioNode } from "@armada/protocol";

beforeEach(() => openExternal.mockClear());

const STUDIO_ID = "01M1STUDIO000000000000001";

/** Main holding one Studio open, with the nodes it carries. */
function holdingStudio(nodes: StudioNode[]): BridgeState {
  const studio: Studio = {
    id: STUDIO_ID,
    manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    created_at: "2026-09-17T09:00:00Z",
    touched_at: "2026-09-17T09:00:00Z",
    nodes,
    edges: [],
  };
  return { ...NOTHING_YET, studio: { state: "read", studio } };
}

const ISSUE = "https://forge.invalid/NickMele/armada/issues/1406";

describe("opening what a Studio node points at", () => {
  it("hands over the address the node carries, and nothing else", async () => {
    const state = holdingStudio([
      { id: "n1", kind: "issue", address: ISSUE, number: "1406", state: "open", position: { x: 0, y: 0 }, created_at: "2026-09-17T09:00:00Z" },
    ]);
    await expect(openStudioNode(state, STUDIO_ID, "n1")).resolves.toEqual({ ok: true });
    expect(openExternal).toHaveBeenCalledWith(ISSUE);
  });

  it("refuses an address that is not a web address, by name", async () => {
    const address = "file:///Users/user/.ssh/id_ed25519";
    const state = holdingStudio([
      { id: "n1", kind: "link", address, position: { x: 0, y: 0 }, created_at: "2026-09-17T09:00:00Z" },
    ]);
    await expect(openStudioNode(state, STUDIO_ID, "n1")).resolves.toEqual({
      ok: false,
      why: "not_addressable",
      address,
    });
    expect(openExternal).not.toHaveBeenCalled();
  });

  it("opens nothing for a node that carries no address, or a Studio it is not holding", async () => {
    const state = holdingStudio([
      { id: "n1", kind: "note", said: "The legend is unreadable", position: { x: 0, y: 0 }, created_at: "2026-09-17T09:00:00Z" },
    ]);
    await expect(openStudioNode(state, STUDIO_ID, "n1")).resolves.toEqual({ ok: false, why: "no_address" });
    await expect(openStudioNode(state, "01M1STUDIO000000000000002", "n1")).resolves.toEqual({
      ok: false,
      why: "no_address",
    });
    expect(openExternal).not.toHaveBeenCalled();
  });
});
