// The finder a walk uses on any page — `walk-find.ts`, which the mock's walks and Bridge's capture window
// both run (`walk-find-page.ts`, built into the isolated world). Here it is pointed at a plain page with
// no Armada code in it: Depot, a stock list with tabs, a field, a button and a status. What it finds
// there is what the owner is ringed on a page of any repository.

import { afterEach, beforeEach, expect, test } from "vitest";

import { find, WALK_UI } from "./walk-find";
import { finder } from "./walk-find-page";

const DEPOT = `
  <h1>Depot</h1>
  <div role="tablist" aria-label="Sections">
    <button role="tab" aria-selected="true" id="t-stock">Stock</button>
    <button role="tab" aria-selected="false" id="t-receiving">Receiving</button>
  </div>
  <table aria-label="Stock"><tbody><tr><td>Washers</td><td>12</td></tr></tbody></table>
  <form id="receiving" aria-label="Receiving form">
    <label>Item <input id="item" value="old"></label>
    <button type="button" id="add">Add item</button>
  </form>
  <p id="status" role="status">Saved</p>
  <p id="words">Twelve washers and a box of bolts, counted</p>
  <section aria-label="Left"><button>Close</button></section>
  <section aria-label="Right"><button>Close</button></section>
  <div hidden><button id="ghost">Ghost</button></div>
  <div ${WALK_UI}><button id="ours">Next</button></div>
`;

let host: HTMLElement;

beforeEach(() => {
  host = document.createElement("div");
  host.innerHTML = DEPOT;
  document.body.append(host);
});

afterEach(() => {
  finder.release();
  host.remove();
});

test("finds a thing by role and by a string inside its accessible name, in any case", () => {
  expect(find({ role: "tab", name: "receiving" })?.id).toBe("t-receiving");
  expect(find({ role: "table", name: "stock" })?.tagName).toBe("TABLE");
  expect(find({ role: "textbox", name: "Item" })?.id).toBe("item");
  expect(find({ role: "button", name: "add" })?.id).toBe("add");
  expect(find({ role: "status" })?.id).toBe("status");
});

test("exact asks for the whole name and nothing longer", () => {
  expect(find({ role: "button", name: "Add", exact: true })).toBeNull();
  expect(find({ role: "button", name: "Add item", exact: true })?.id).toBe("add");
});

test("finds words by the smallest element that draws them, and not a parent that holds them", () => {
  expect(find({ role: "text", name: "box of bolts" })?.id).toBe("words");
});

test("looks only inside what `within` names, and takes the last of several", () => {
  expect(find({ role: "button", name: "Close", within: { role: "region", name: "Left" } })?.closest("section")?.ariaLabel).toBe("Left");
  expect(find({ role: "button", name: "Close" })?.closest("section")?.ariaLabel).toBe("Right");
  expect(find({ role: "button", name: "Close", within: { role: "region", name: "Nowhere" } })).toBeNull();
});

test("takes the first of the places a thing is drawn in that is on the page, for a layout that differs", () => {
  const back = { role: "button", name: "Back to Stock" };
  expect(find({ ...back, or: [{ role: "tab", name: "Receiving" }] })?.id).toBe("t-receiving");
  expect(find({ role: "button", name: "Add item", or: [{ role: "tab", name: "Receiving" }] })?.id).toBe("add");
  expect(find({ ...back, or: [{ role: "tab", name: "Nowhere" }] })).toBeNull();
});

test("never finds what is hidden, or the walk's own ring and card", () => {
  expect(find({ role: "button", name: "Ghost" })).toBeNull();
  expect(find({ role: "button", name: "Next" })).toBeNull();
});

test("the page entry answers with a box on screen and holds the element for what follows", async () => {
  const box = await finder.arrive({ role: "button", name: "Add item" }, 1000);
  expect(box).not.toBeNull();
  const real = document.getElementById("add")!.getBoundingClientRect();
  expect(box!.width).toBe(Math.round(real.width));
  expect(finder.box()).toEqual(box);
});

test("it waits for a target that arrives late and gives up on one that never does", async () => {
  const late = finder.arrive({ role: "button", name: "Delayed" }, 2000);
  setTimeout(() => {
    const button = document.createElement("button");
    button.textContent = "Delayed";
    host.append(button);
  }, 250);
  expect(await late).not.toBeNull();
  const before = Date.now();
  expect(await finder.arrive({ role: "button", name: "Never" }, 300)).toBeNull();
  expect(Date.now() - before).toBeGreaterThanOrEqual(250);
  expect(finder.box()).toBeNull();
});

test("focus gives the keyboard to the held field and says what it is", async () => {
  await finder.arrive({ role: "textbox", name: "Item" }, 1000);
  expect(finder.focus()).toBe("field");
  expect(document.activeElement?.id).toBe("item");
  await finder.arrive({ role: "button", name: "Add item" }, 1000);
  expect(finder.focus()).toBe("other");
});

test("words answers where a string starts and ends inside the held element, and null where it is not there", async () => {
  await finder.arrive({ role: "text", name: "box of bolts" }, 1000);
  const across = finder.words("box of bolts");
  expect(across).not.toBeNull();
  expect(across!.to.x).toBeGreaterThan(across!.from.x);
  expect(finder.words("a sentence that is not there")).toBeNull();
});

test("it writes nothing to the page it reads", async () => {
  const before = host.innerHTML;
  await finder.arrive({ role: "tab", name: "Receiving" }, 1000);
  finder.words("x");
  expect(host.innerHTML).toBe(before);
});

test("a held element that left the page has no box, and release forgets it", async () => {
  await finder.arrive({ role: "button", name: "Add item" }, 1000);
  document.getElementById("add")!.remove();
  expect(finder.box()).toBeNull();
  await finder.arrive({ role: "tab", name: "Stock" }, 1000);
  finder.release();
  expect(finder.box()).toBeNull();
});
