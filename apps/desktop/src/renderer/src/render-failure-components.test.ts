// armada-21wq: a copied render error carried the JS stack as its chain, which
// names no component, so the component stack shown on screen was lost from it.

import { expect, test } from "vitest";
import { rendererFailure } from "@armada/shell";
import type { BridgeIdentity } from "@armada/protocol";

const bridge = {
  auditPath: null,
  fleetProtocol: null,
} as unknown as BridgeIdentity;

const where = `
    at Dialog (http://localhost/Dialog.tsx:120:3)
    at FixDialog (http://localhost/FixDialog.tsx:40:3)
    at MainRedBand (http://localhost/MainRedBand.tsx:12:3)`;

test("the copied render error names every component on the stack, innermost first", () => {
  const failure = rendererFailure(
    {
      message: "Maximum update depth exceeded",
      component: "Dialog",
      where,
      stack: "Error: x\n    at f (a.js:1:1)",
    },
    "the overview",
    bridge,
    true,
  );
  expect(failure.payload.fields).toContainEqual({
    key: "components",
    value: "Dialog < FixDialog < MainRedBand",
  });
});

test("no components line when React gave no component stack", () => {
  const failure = rendererFailure(
    {
      message: "x",
      component: null,
      where: null,
      stack: null,
    },
    "the overview",
    bridge,
    true,
  );
  expect((failure.payload.fields ?? []).map((field) => field.key)).not.toContain("components");
});
