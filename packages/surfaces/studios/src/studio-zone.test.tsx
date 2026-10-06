// A read-in in its Zone, through `App` — #1620, decided with the owner on 2 Oct 2026: everything the
// read-in brings back lands inside one Zone with one line to it from the issue, a Cluster is a box
// round its Notes, dragging the Zone carries what is in it, and a Zone is put down from the rail.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted, READ_IN_NAME, zoning, ZONE_PROPOSAL_NAME, zoneProposing } from "@armada/desktop/mock";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

const node = (name: RegExp | string) => page.getByRole("group", { name, exact: typeof name === "string" });
const box = (name: RegExp | string) => node(name).element().getBoundingClientRect();
/** Whether `inner` is drawn wholly inside `outer`. */
const holds = (outer: DOMRect, inner: DOMRect) =>
  inner.left >= outer.left && inner.right <= outer.right && inner.top >= outer.top && inner.bottom <= outer.bottom;

/** A pointer drag, the way React Flow hears one: down on the node, moves and up on the window. */
function drag(element: Element, dx: number, dy: number): void {
  const from = element.getBoundingClientRect();
  const at = (x: number, y: number) => ({
    bubbles: true,
    view: window,
    clientX: from.x + from.width / 2 + x,
    clientY: from.y + 12 + y,
    button: 0,
    buttons: 1,
  });
  element.dispatchEvent(new MouseEvent("mousedown", at(0, 0)));
  for (const step of [0.1, 0.5, 1]) window.dispatchEvent(new MouseEvent("mousemove", at(dx * step, dy * step)));
  window.dispatchEvent(new MouseEvent("mouseup", at(dx, dy)));
}

/** The read-in's Studio, open and Continued, so it is the person's to move. */
async function openEditable() {
  const fleet = zoning();
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: READ_IN_NAME, exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(node("Zone")).toBeVisible();
  return fleet;
}

const kept = (fleet: ReturnType<typeof zoning>, id: string) => fleet.studios()[0]!.nodes.find((one) => one.id === id)!;

test("a read-in is one Zone with one line to it, and each Cluster is a box round its Notes", async () => {
  await openEditable();
  const zone = box("Zone");
  for (const cluster of ["The problem today", "What to build", "Risks to watch"]) {
    expect(holds(zone, box(`Cluster: ${cluster}`)), `${cluster} is inside the Zone`).toBe(true);
  }
  const risks = box("Cluster: Risks to watch");
  expect(holds(risks, box(/^Note: Watch for a `scope` change/))).toBe(true);
  expect(holds(risks, box(/^Note: Watch for an edit after the gate/))).toBe(true);
  // A Cluster of one Note is not drawn: its Note is loose in the Zone (2 Oct 2026).
  await expect.element(node("Cluster: Acceptance")).not.toBeInTheDocument();
  expect(holds(zone, box(/^Note: Done when, on a failed task/))).toBe(true);
  expect(holds(zone, box(/^Finding: Read in/))).toBe(true);
  expect(holds(zone, box(/^Issue: Nobody can change a task/)), "the issue stays outside what it made").toBe(false);

  // One line out of the issue, to the Zone, where the read-in drew eighteen — and none from a
  // Note to the Cluster round it.
  const lines = [...document.querySelectorAll("[aria-label]")]
    .map((one) => one.getAttribute("aria-label")!)
    .filter((said) => said.includes(" produced "));
  expect(lines).toEqual(["Issue Nobody can change a task's title, brief, files, done-when or model produced Zone"]);
});

test("dragging the Zone moves everything in it, and its Notes stay where they are in it", async () => {
  const fleet = await openEditable();
  const zoneWas = kept(fleet, "read-in-zone").position;
  const noteWas = kept(fleet, "read-in-risks-note-2");
  const drawnWas = box(/^Note: Watch for a `scope` change/);
  const frameWas = box("Zone");

  drag(node("Zone").element(), 0, 160);
  await expect.poll(() => kept(fleet, "read-in-zone").position.y).toBeGreaterThan(zoneWas.y + 60);

  // One write: the Zone moved, and what it holds is still where it was in it.
  expect(kept(fleet, "read-in-risks-note-2")).toEqual(noteWas);
  const drawn = box(/^Note: Watch for a `scope` change/);
  const frame = box("Zone");
  expect(Math.round(drawn.top - drawnWas.top)).toBe(Math.round(frame.top - frameWas.top));
  expect(holds(box("Cluster: Risks to watch"), drawn)).toBe(true);
});

test("a Zone is put down from the rail, and a Note dropped on it goes in", async () => {
  const fleet = await openEditable();
  const zones = () => fleet.studios()[0]!.nodes.filter((one) => one.kind === "zone");

  await page.getByRole("button", { name: "Add a Zone" }).click();
  // **React Flow's pane has no role**, so it is found by its class, and the press lands on a
  // point checked to be the pane itself rather than a node over it.
  const pane = document.querySelector<HTMLElement>(".armada-studio-whiteboard .react-flow__pane")!;
  const loose = box(/^Note: Ask whether a Judge/);
  const area = pane.getBoundingClientRect();
  let at: { x: number; y: number } | undefined;
  for (let y = area.bottom - 80; at === undefined && y > area.top; y -= 40) {
    for (let x = area.left + 120; at === undefined && x < area.right - 120; x += 40) {
      if (document.elementFromPoint(x, y) === pane) at = { x, y };
    }
  }
  await userEvent.click(page.elementLocator(pane), { position: { x: at!.x - area.left, y: at!.y - area.top } });
  await expect.poll(() => zones().length).toBe(2);
  const added = zones().find((one) => one.id !== "read-in-zone")!;

  await expect.poll(() => node("Zone").elements().length).toBe(2);
  const empty = node("Zone").elements().map((one) => one.getBoundingClientRect()).find((one) => !holds(one, box(/^Finding/)))!;
  drag(node(/^Note: Ask whether a Judge/).element(), empty.left + empty.width / 2 - (loose.left + loose.width / 2), empty.top + 60 - (loose.top + 12));
  await expect.poll(() => kept(fleet, "read-in-loose").within).toBe(added.id);
});

/** Whether two boxes share any area. */
const overlap = (a: DOMRect, b: DOMRect) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

test("a proposed relation is a dot on its line that covers no card, and opens to be answered", async () => {
  const fleet = await openEditable();
  const named = /^Helm proposes: Note Done when, on a failed task/;
  const dot = page.getByRole("button", { name: named });
  await expect.element(dot).toBeVisible();

  // The owner, 2 Oct 2026: the card sat on "Watch for a `scope` change" and hid its text. Shut, the
  // proposal draws nothing over the board but its dot, and the dot sits on no card.
  expect(page.getByRole("group", { name: named }).query()).toBeNull();
  const drawn = [...document.querySelectorAll(".armada-studio-edge__proposal-at")].map((one) => one.getBoundingClientRect());
  const cards = page.getByRole("group", { name: /^(Note|Finding|Contradiction|Issue): / }).elements();
  for (const card of cards) {
    for (const label of drawn) expect(overlap(label, card.getBoundingClientRect()), card.getAttribute("aria-label")!).toBe(false);
  }

  await dot.hover();
  const proposal = page.getByRole("group", { name: named });
  await expect.element(proposal.getByText("blocks", { exact: true })).toBeVisible();
  await proposal.getByRole("button", { name: /^Reject: / }).click();
  await expect.poll(() => fleet.studios()[0]!.edges.some((edge) => edge.id === "read-in-blocks" && edge.standing === "proposed")).toBe(false);
});

test("a proposal whose line runs across a Note moves its dot along the line, off the Note", async () => {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(zoneProposing().scenario, host), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: ZONE_PROPOSAL_NAME, exact: true }).click();

  // The line from the first Note to the third runs straight across the second, and its middle is on it.
  const dot = page.getByRole("button", { name: /^You proposed: Note The legend wraps at 720 wide blocks / });
  await expect.element(dot).toBeVisible();
  const crossed = box(/^Note: The step bar's legend is unreadable/);
  const at = dot.element().getBoundingClientRect();
  expect(overlap(at, crossed), "the dot sits on the Note its line crosses").toBe(false);
  // Still on its own line, which runs at the height of the Notes it joins.
  const from = box(/^Note: The legend wraps at 720 wide/);
  expect(at.top + at.height / 2).toBeGreaterThan(from.top);
  expect(at.top + at.height / 2).toBeLessThan(from.bottom);
});

/**
 * A press on a frame's own ground — inside it, over none of the cards it holds — the way a pointer
 * makes one. **Read off what the board draws there**, so the press lands on the frame and not on a
 * card that happens to sit at its middle.
 */
async function pressGround(frame: Element): Promise<void> {
  const area = frame.getBoundingClientRect();
  let at: { x: number; y: number } | undefined;
  for (let y = area.bottom - 16; at === undefined && y > area.top + 16; y -= 12) {
    for (let x = area.left + 16; at === undefined && x < area.right - 16; x += 12) {
      if (document.elementFromPoint(x, y)?.closest(".react-flow__node") === frame) at = { x, y };
    }
  }
  expect(at, "the frame has ground of its own to press").toBeDefined();
  await userEvent.click(page.elementLocator(frame), { position: { x: at!.x - area.left, y: at!.y - area.top } });
}

const said = (fleet: ReturnType<typeof zoning>, words: string) =>
  fleet.studios()[0]!.nodes.find((one) => one.kind === "note" && one.said === words);

test("a kind armed and pressed inside the Zone lands in it", async () => {
  const fleet = await openEditable();

  await page.getByRole("button", { name: "Add a Note", exact: true }).click();
  await pressGround(node("Zone").element());
  await userEvent.fill(page.getByLabelText("Note", { exact: true }), "Pressed inside the Zone");
  await page.getByRole("button", { name: "Add note" }).click();

  await expect.poll(() => said(fleet, "Pressed inside the Zone")?.within).toBe("read-in-zone");
  // Fleet holding it is not the board drawing it: the publish renders after the write answers.
  await expect.element(node(/^Note: Pressed inside the Zone/)).toBeVisible();
  expect(holds(box("Zone"), box(/^Note: Pressed inside the Zone/))).toBe(true);
});

// A Cluster's Notes are its record, made by grouping, so it takes nothing by a press either.
test("a press inside a Cluster lands in the Zone round it", async () => {
  const fleet = await openEditable();

  await page.getByRole("button", { name: "Add a Note", exact: true }).click();
  await pressGround(node("Cluster: The problem today").element());
  await userEvent.fill(page.getByLabelText("Note", { exact: true }), "Pressed inside a Cluster");
  await page.getByRole("button", { name: "Add note" }).click();
  await expect.poll(() => said(fleet, "Pressed inside a Cluster")?.within).toBe("read-in-zone");
});

// A Zone holds any kind but another Zone.
test("a Zone pressed inside a Zone lands on the board", async () => {
  const fleet = await openEditable();
  const zones = () => fleet.studios()[0]!.nodes.filter((one) => one.kind === "zone");
  await page.getByRole("button", { name: "Add a Zone" }).click();
  await pressGround(node("Zone").element());
  await expect.poll(() => zones().length).toBe(2);
  expect(zones().find((one) => one.id !== "read-in-zone")?.within).toBeUndefined();
});
