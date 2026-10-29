// What a Job's agent reads of its page: roles, names, values and state, one line a part, cut where it runs long.

import { afterEach, expect, test } from "vitest";

import { snapshotOf } from "./snapshot";

let root: HTMLElement | undefined;
afterEach(() => root?.remove());

function page(markup: string): HTMLElement {
  root = document.createElement("main");
  root.innerHTML = markup;
  document.body.append(root);
  return root;
}

test("reads each part by its role and name, with its value and state, and its words once", () => {
  const at = page(`
    <h1>Checkout</h1>
    <label>Discount code <input value="SPRING10" /></label>
    <button disabled>Apply</button>
    <label><input type="checkbox" checked /> Gift wrap</label>
    <p>Total <strong>$84.00</strong></p>
    <div hidden><button>Secret</button></div>
  `);
  expect(snapshotOf(at, 10_000).split("\n")).toEqual([
    '- main',
    '  - heading "Checkout" [level=1]',
    '  - text: "Discount code"',
    '  - textbox "Discount code": "SPRING10"',
    '  - button "Apply" [disabled]',
    '  - checkbox "Gift wrap" [checked]',
    '  - text: "Gift wrap"',
    '  - text: "Total"',
    '  - text: "$84.00"',
  ]);
});

test("is cut at the length it is given, and says so", () => {
  const at = page([...Array(200).keys()].map((one) => `<button>Row ${one}</button>`).join(""));
  const read = snapshotOf(at, 500);
  expect(read.length).toBeLessThanOrEqual(500 + "\n- (cut: the page goes on)".length);
  expect(read.endsWith("- (cut: the page goes on)")).toBe(true);
});
