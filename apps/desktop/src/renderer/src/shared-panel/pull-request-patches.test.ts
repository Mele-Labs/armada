import { expect, test } from "vitest";
import type { PullReviewThread } from "@armada/protocol";

import { excerptsOf } from "./pull-request-patches";

const PATCH = [
  "@@ -38,6 +38,9 @@",
  "   const subtotal = lines.reduce((sum, line) => sum + line.price * line.quantity, 0);",
  "   const discount = discountOf(cart);",
  "-  return Math.round(subtotal) - discount;",
  "+  const credit = giftCards.reduce((sum, card) => sum + card.balance, 0);",
  "+  return Math.round(subtotal) - discount - credit;",
  "   }",
  "   export function tax() {",
  "   return 0;",
  "   }",
].join("\n");

const thread = (line: number | undefined, path = "src/cart/totals.ts"): PullReviewThread => ({
  id: "t",
  path,
  ...(line === undefined
    ? {}
    : {
        line,
      }),
  side: "right",
  resolved: false,
  comments: [],
});

const read = {
  ok: true as const,
  files: [
    {
      manifest_id: "web",
      number: 812,
      path: "src/cart/totals.ts",
      patch: PATCH,
    },
  ],
};

test("a thread on a line draws that line and a few either side, under a header numbered where the cut begins", () => {
  const excerpt = excerptsOf(read)!(thread(41));
  expect(excerpt?.path).toBe("src/cart/totals.ts");
  expect(excerpt?.lines.map((one) => one.text)).toEqual([
    "@@ -39,5 +39,6 @@",
    "   const discount = discountOf(cart);",
    "-  return Math.round(subtotal) - discount;",
    "+  const credit = giftCards.reduce((sum, card) => sum + card.balance, 0);",
    "+  return Math.round(subtotal) - discount - credit;",
    "   }",
    "   export function tax() {",
    "   return 0;",
  ]);
});

test("a thread on a file, on a line the patch does not hold, or on a file not read draws none", () => {
  const excerpt = excerptsOf(read)!;
  expect(excerpt(thread(undefined))).toBeUndefined();
  expect(excerpt(thread(200))).toBeUndefined();
  expect(excerpt(thread(41, "README.md"))).toBeUndefined();
});

test("no patches read, or a refused read, draws no excerpt at all", () => {
  expect(excerptsOf(undefined)).toBeUndefined();
  expect(
    excerptsOf({
      ok: false,
      said: "The forge did not answer.",
    }),
  ).toBeUndefined();
});
