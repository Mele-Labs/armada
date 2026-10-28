// A proposed change as the steps it touches, matched by id: what is removed, added or changed, where each sits, and the
// names of the rest. The card each becomes is `workflow-manager.test.tsx`.

import { stepChangesOf } from "@armada/components/src/compositions/WorkflowManager/diff";
import type { ManagedStep } from "@armada/components";
import { expect, test } from "vitest";

const step = (id: string, over: Partial<ManagedStep> = {}): ManagedStep => ({ id, label: id.toUpperCase(), ends: `${id} ends`, words: `${id} words`, ...over });

const kinds = (before: readonly ManagedStep[], after: readonly ManagedStep[]) => stepChangesOf(before, after).changes.map((one) => `${one.kind}:${one.n}:${(one.kind === "removed" ? one.before : one.after).id}`);

test("a step only in the new workflow is added, one only in the old is removed, and one in both and the same is only named", () => {
  const out = stepChangesOf([step("a"), step("b")], [step("a"), step("c")]);
  expect(out.changes).toEqual([
    { kind: "removed", n: 2, before: step("b") },
    { kind: "added", n: 2, after: step("c") },
  ]);
  expect(out.unchanged).toEqual(["A"]);
});

test("a step matched by id whose facts differ is changed, with both sides", () => {
  const out = stepChangesOf([step("a", { words: "old" })], [step("a", { words: "new" })]);
  expect(out.changes).toEqual([{ kind: "changed", n: 1, was: 1, before: step("a", { words: "old" }), after: step("a", { words: "new" }) }]);
  expect(out.unchanged).toEqual([]);
});

test("a change to only what ends a step, or only when it is passed over, is a change", () => {
  expect(kinds([step("a")], [step("a", { ends: "you approve" })])).toEqual(["changed:1:a"]);
  expect(kinds([step("a")], [step("a", { passedOver: "nothing is seen" })])).toEqual(["changed:1:a"]);
  expect(kinds([step("a", { passedOver: "nothing is seen" })], [step("a")])).toEqual(["changed:1:a"]);
});

test("a step with no passed-over line is the same as one with an empty one, and its glyph does not make it differ", () => {
  expect(stepChangesOf([step("a")], [step("a", { passedOver: "" })])).toEqual({ changes: [], unchanged: ["A"] });
  expect(stepChangesOf([step("a")], [step("a", { glyph: "walk" })]).changes).toEqual([]);
});

test("a change to a step's name alone is a change", () => {
  expect(kinds([step("a")], [step("a", { label: "Renamed" })])).toEqual(["changed:1:a"]);
});

test("a workflow set against itself touches nothing and names every step", () => {
  const same = [step("a"), step("b"), step("c")];
  expect(stepChangesOf(same, same)).toEqual({ changes: [], unchanged: ["A", "B", "C"] });
});

test("changes read in the order of the new workflow, a removed step where it stood", () => {
  const before = [step("a"), step("gone-1"), step("gone-2"), step("b"), step("last")];
  const after = [step("a"), step("new"), step("b", { words: "changed" })];
  // `n` is the place in the workflow the step is in: the new one for what stays, the old one for what went.
  expect(kinds(before, after)).toEqual(["removed:2:gone-1", "removed:3:gone-2", "added:2:new", "changed:3:b", "removed:5:last"]);
});

test("a step removed from the front reads first, and from an otherwise empty result, alone", () => {
  expect(kinds([step("x"), step("a")], [step("a")])).toEqual(["removed:1:x"]);
  expect(kinds([step("x"), step("a")], [step("y"), step("a")])).toEqual(["removed:1:x", "added:1:y"]);
  expect(stepChangesOf([step("x")], [])).toEqual({ changes: [{ kind: "removed", n: 1, before: step("x") }], unchanged: [] });
  expect(stepChangesOf([], [step("x")])).toEqual({ changes: [{ kind: "added", n: 1, after: step("x") }], unchanged: [] });
});

test("a step that only moves is changed, with the place it left and the place it took", () => {
  const out = stepChangesOf([step("discuss"), step("walk"), step("land")], [step("walk"), step("discuss"), step("land")]);
  // Walk is the one said to move: the earlier step stays put where two readings tie.
  expect(out.changes).toEqual([{ kind: "changed", n: 1, was: 2, before: step("walk"), after: step("walk") }]);
  expect(out.unchanged).toEqual(["DISCUSS", "LAND"]);
});

test("a step moved a long way is the one changed, not each step it passed", () => {
  expect(kinds([step("a"), step("b"), step("c"), step("d")], [step("d"), step("a"), step("b"), step("c")])).toEqual(["changed:1:d"]);
});

test("a step that only shifted because another was added or removed is not moved", () => {
  expect(kinds([step("a"), step("b")], [step("new"), step("a"), step("b")])).toEqual(["added:1:new"]);
  expect(kinds([step("x"), step("a"), step("b")], [step("a"), step("b")])).toEqual(["removed:1:x"]);
  expect(stepChangesOf([step("a"), step("b")], [step("new"), step("a"), step("b")]).unchanged).toEqual(["A", "B"]);
});
