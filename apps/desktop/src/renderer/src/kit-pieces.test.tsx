// Remove on a skill or plugin row and Undo for a skill, as `useKit` folds Fleet's answers: each act draws the inventory
// Fleet answers with, and a refusal reads the inventory again, so a row the window had out of date says what is true.
// Browser: a probe draws what the hook holds, over a fake slice.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { KitInventory, Outcome } from "@armada/protocol";
import { useKit, type KitSlice } from "@armada/screens/src/manifest-kit";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const inventory = (...skills: string[]): KitInventory =>
  ({ harness: "claude", home: "/Users/user/.claude", kinds: [{ kind: "skills", read: { what: "read", items: skills.map((name) => ({ name, says: "", source: `/Users/user/.claude/skills/${name}` })) } }] }) as unknown as KitInventory;
const REFUSED: Outcome = { ok: false, why: "not_connected" };
const TRASHED = "/Users/user/.Trash/humanizer";

let hook: ReturnType<typeof useKit> | undefined;
function Probe({ slice }: { slice: KitSlice }) {
  hook = useKit(slice);
  return null;
}

function slice(over: Partial<KitSlice>): KitSlice & { onReadKitInventory: ReturnType<typeof vi.fn> } {
  const refused = async () => ({ ok: false as const, outcome: REFUSED });
  return {
    onReadKitInventory: vi.fn(async () => ({ ok: true as const, setup: inventory("humanizer", "notes") })),
    onRemoveKitAllowedCommand: refused,
    onListKitServers: refused,
    onAddKitServer: refused,
    onForgetKitServer: refused,
    onSetKitServerReach: refused,
    onSetManifestServerReach: refused,
    onRemoveKitPiece: refused,
    onRestoreKitPiece: refused,
    ...over,
  } as KitSlice & { onReadKitInventory: ReturnType<typeof vi.fn> };
}

const roots: Root[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
  hook = undefined;
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount(kit: KitSlice) {
  const root = createRoot(document.createElement("div"));
  roots.push(root);
  act(() => root.render(<Probe slice={kit} />));
  await flush();
}

const names = () => hook?.setup?.kinds.flatMap((one) => (one.read.what === "read" ? one.read.items.map((item) => item.name) : []));

test("a skill removed draws the inventory Fleet answers with, and Undo puts it back by the place Fleet trashed it", async () => {
  const restore = vi.fn(async () => ({ ok: true as const, setup: inventory("humanizer", "notes") }));
  const kit = slice({
    onRemoveKitPiece: async () => ({ ok: true, removed: { setup: inventory("notes"), trashed: TRASHED } }),
    onRestoreKitPiece: restore,
  });
  await mount(kit);

  act(() => hook!.pieces.onRemove({ kind: "skill", name: "humanizer" }));
  await flush();
  expect(names()).toEqual(["notes"]);
  expect(hook!.pieces.trashed).toEqual({ name: "humanizer" });

  act(() => hook!.pieces.onUndo());
  await flush();
  expect(restore).toHaveBeenCalledWith({ trashed: TRASHED });
  expect(names()).toEqual(["humanizer", "notes"]);
  expect(hook!.pieces.trashed).toBeNull();
});

test("a remove Fleet refuses says so and reads the inventory again", async () => {
  const kit = slice({});
  await mount(kit);
  expect(kit.onReadKitInventory).toHaveBeenCalledTimes(1);

  act(() => hook!.pieces.onRemove({ kind: "plugin", name: "impeccable@impeccable" }));
  await flush();
  expect(hook!.refused).toEqual(REFUSED);
  expect(kit.onReadKitInventory).toHaveBeenCalledTimes(2);
});

test("an Undo Fleet refuses says so, reads the inventory again and offers Undo no longer", async () => {
  const kit = slice({ onRemoveKitPiece: async () => ({ ok: true, removed: { setup: inventory("notes"), trashed: TRASHED } }) });
  await mount(kit);
  act(() => hook!.pieces.onRemove({ kind: "skill", name: "humanizer" }));
  await flush();

  act(() => hook!.pieces.onUndo());
  await flush();
  expect(hook!.refused).toEqual(REFUSED);
  expect(kit.onReadKitInventory).toHaveBeenCalledTimes(2);
  expect(hook!.pieces.trashed).toBeNull();
});
