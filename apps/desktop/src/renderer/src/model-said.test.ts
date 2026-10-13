// How a model is named in a Session's head: from the full id its process resolved, from the alias that was
// picked, and as it came where it is neither. Nothing is made up for a model that was not said.

import { expect, test } from "vitest";

import { modelSaid } from "@armada/components/src/compositions/SessionFrame/ModelChip";

test("A resolved id is named by its family and version", () => {
  expect(modelSaid("vendor-opus-5-5")).toBe("Opus 5.5");
  expect(modelSaid("vendor-sonnet-5-5")).toBe("Sonnet 5.5");
  expect(modelSaid("vendor-opus-4-5-20251101")).toBe("Opus 4.5");
  expect(modelSaid("vendor-sonnet-4-20250514")).toBe("Sonnet 4");
  expect(modelSaid("vendor-3-5-sonnet-20241022")).toBe("Sonnet 3.5");
});

test("A picked alias is capitalised and not given a version", () => {
  expect(modelSaid("opus")).toBe("Opus");
  expect(modelSaid("sonnet")).toBe("Sonnet");
  expect(modelSaid("haiku")).toBe("Haiku");
});

test("A model the reader does not know is shown as it came", () => {
  expect(modelSaid("stand-in")).toBe("stand-in");
});
