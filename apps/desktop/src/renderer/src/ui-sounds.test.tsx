// Interface sounds, against a counting AudioContext: a key that moves a list's mark plays once, a key that
// moves nothing plays nothing, a press on a control plays, and the switch off silences both.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installUiSounds, setUiSounds } from "./ui-sounds";

let played = 0;

class CountingContext {
  readonly sampleRate = 48000;
  readonly state = "running";
  readonly destination = {};
  resume = async () => {};
  createBuffer = (_channels: number, length: number) => ({ length, copyToChannel: () => {} });
  createGain = () => ({ gain: { value: 0 }, connect: (next: unknown) => next });
  createBufferSource = () => ({ buffer: null, connect: (next: unknown) => next, start: () => (played += 1) });
}

const frame = () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
// Past the gap a run of sounds is held to, so one test's last sound never swallows the next one's first.
const pastTheGap = () => new Promise((done) => setTimeout(done, 40));

/** Two rows, the first marked, with a key handler that moves the mark on j the way Bridge's lists do. */
function list(): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = '<div role="row" data-selected>one</div><div role="row">two</div><button>Press</button>';
  root.addEventListener("keydown", (event) => {
    if (event.key !== "j") return;
    const [one, two] = root.querySelectorAll('[role="row"]');
    one!.removeAttribute("data-selected");
    two!.setAttribute("data-selected", "");
  });
  document.body.append(root);
  return root;
}

let uninstall: () => void = () => {};

beforeEach(async () => {
  vi.stubGlobal("AudioContext", CountingContext);
  await pastTheGap();
  played = 0;
  setUiSounds(true);
  uninstall = installUiSounds();
});

afterEach(() => {
  uninstall();
  document.body.innerHTML = "";
  vi.unstubAllGlobals();
});

describe("interface sounds", () => {
  it("a key that moves the selection plays once", async () => {
    const root = list();
    root.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
    await frame();
    expect(played).toBe(1);
  });

  it("a key that moves nothing plays nothing", async () => {
    const root = list();
    root.dispatchEvent(new KeyboardEvent("keydown", { key: "k", bubbles: true }));
    await frame();
    expect(played).toBe(0);
  });

  it("a press on a control plays", () => {
    list().querySelector("button")!.click();
    expect(played).toBe(1);
  });

  it("off is silent for a move and a press", async () => {
    setUiSounds(false);
    const root = list();
    root.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
    await frame();
    root.querySelector("button")!.click();
    expect(played).toBe(0);
  });
});
