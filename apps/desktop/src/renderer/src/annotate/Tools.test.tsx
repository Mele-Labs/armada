// The note card's tools in a real window: what each one does to the element on the page, what the list
// of changes says, and that closing the card gives the page back. And the pictures a card takes.

import { useState, type CSSProperties } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { isPng, type StoryIndex, type StyleChange } from "../../../shared/annotations";
import "../styles/index.css";
import "./annotate.css";
import { computed } from "./restyle";
import type { Sink } from "./sink";
import type { StoryModule } from "./story";
import { asPng, Examples, imagesIn, Tools } from "./Tools";

/** The eight bytes a PNG starts with, and one more. */
const PNG = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 0]);

let mounted: { root: Root; host: HTMLElement }[] = [];
const onPage: HTMLElement[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  for (const one of onPage.splice(0)) one.remove();
  document.documentElement.removeAttribute("style");
  sessionStorage.clear();
});

/** An element the note is on: 13px, white on nothing, with two lines of words. */
function target(html = "<b>Fix the flaky test</b>"): HTMLElement {
  const element = document.createElement("div");
  element.setAttribute("data-target", "");
  element.style.cssText = "font-size: 13px; color: rgb(255, 255, 255); padding: 0px";
  element.innerHTML = html;
  document.body.append(element);
  onPage.push(element);
  return element;
}

/** The theme's tokens the tools offer, at values easy to tell apart. */
function theme(): void {
  const root = document.documentElement.style;
  root.setProperty("--text-lg", "20px");
  root.setProperty("--text-xl", "24px");
  root.setProperty("--fg-default", "rgb(10, 20, 30)");
  root.setProperty("--accent", "rgb(74, 158, 219)");
  root.setProperty("--bg-raised", "rgb(1, 2, 3)");
  root.setProperty("--space-2", "8px");
  root.setProperty("--space-4", "16px");
}

/** The tools as the card holds them, with the list of changes kept as the card keeps it. */
function Card({ element, seen }: { element: Element | null; seen: (changes: StyleChange[]) => void }) {
  const [changes, setChanges] = useState<StyleChange[]>([]);
  return (
    <Tools
      id="20260916-222105-k3f9"
      element={element}
      changes={changes}
      onChanges={(next) => {
        setChanges(next);
        seen(next);
      }}
      keep="test:text"
    />
  );
}

async function tools(element: Element | null): Promise<{ changes: () => StyleChange[]; unmount: () => void }> {
  theme();
  let latest: StyleChange[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  root.render(<Card element={element} seen={(next) => (latest = next)} />);
  mounted.push({ root, host });
  await expect.element(page.getByRole("toolbar", { name: "Tools" })).toBeVisible();
  return {
    changes: () => latest,
    unmount: () => {
      root.unmount();
      mounted = mounted.filter((one) => one.root !== root);
    },
  };
}

const press = (name: string) => page.getByRole("button", { name, exact: true }).click();
const changes = () => page.getByRole("list", { name: "Changes" });

describe("the toolbar", () => {
  test("offers Size, Colour, Fill, Spacing and Text on an element with words", async () => {
    await tools(target());
    for (const name of ["Size", "Colour", "Fill", "Spacing", "Text"]) await expect.element(page.getByRole("button", { name, exact: true })).toBeEnabled();
  });

  test("leaves Text off on an element with no words, and says why", async () => {
    await tools(target("<span></span>"));
    await expect.element(page.getByRole("button", { name: "Text", exact: true })).toBeDisabled();
    await expect.element(page.getByRole("button", { name: "Size", exact: true })).toBeEnabled();
    expect(document.body.textContent).toContain("Text: no words on it");
  });

  test("offers nothing where the element is gone from the page", async () => {
    await tools(null);
    for (const name of ["Size", "Colour", "Fill", "Spacing", "Text"]) await expect.element(page.getByRole("button", { name, exact: true })).toBeDisabled();
  });

  test("moves between its buttons with the arrow keys, as one stop on Tab", async () => {
    await tools(target());
    (page.getByRole("button", { name: "Size", exact: true }).element() as HTMLElement).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement).toBe(page.getByRole("button", { name: "Colour", exact: true }).element());
    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
    expect(document.activeElement).toBe(page.getByRole("button", { name: "Text", exact: true }).element());
  });
});

describe("a pick", () => {
  test("Size shows on the element at once and is listed with what it was, what it is and the token", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    expect(computed(element, "font-size")).toBe("20px");
    // What is inside it moves too: the area he picked is what gets larger.
    expect(computed(element.querySelector("b")!, "font-size")).toBe("20px");
    expect(held.changes()).toEqual([{ kind: "size", property: "font-size", before: "13px", after: "20px", token: "--text-lg" }]);
    const row = changes().getByRole("listitem");
    await expect.element(row).toHaveTextContent("Size");
    await expect.element(row).toHaveTextContent("13px → 20px");
    await expect.element(row).toHaveTextContent("--text-lg");
    await expect.element(page.getByRole("button", { name: "--text-lg" })).toHaveAttribute("aria-pressed", "true");
  });

  test("a second pick replaces the first and still remembers what the page drew before either", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    await press("--text-xl");
    expect(computed(element, "font-size")).toBe("24px");
    expect(held.changes()).toEqual([{ kind: "size", property: "font-size", before: "13px", after: "24px", token: "--text-xl" }]);
    await expect.element(page.getByRole("button", { name: "--text-lg" })).toHaveAttribute("aria-pressed", "false");
  });

  test("pressing the picked token again undoes it", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    await press("--text-lg");
    expect(held.changes()).toEqual([]);
    expect(computed(element, "font-size")).toBe("13px");
    await expect.element(changes()).not.toBeInTheDocument();
  });

  test("Colour and Fill are read as the page computes them and written as the tokens are", async () => {
    const element = target();
    const held = await tools(element);
    await press("Colour");
    await press("--accent");
    await press("Fill");
    await press("--bg-raised");
    expect(held.changes()).toEqual([
      { kind: "colour", property: "color", before: "#FFFFFF", after: "#4A9EDB", token: "--accent" },
      { kind: "fill", property: "background-color", before: "transparent", after: "#010203", token: "--bg-raised" },
    ]);
    expect(computed(element, "color")).toBe("rgb(74, 158, 219)");
    expect(computed(element, "background-color")).toBe("rgb(1, 2, 3)");
  });

  test("Spacing sets Padding and Gap apart, each its own row", async () => {
    const element = target();
    const held = await tools(element);
    await press("Spacing");
    const padding = page.getByRole("group", { name: "Padding" });
    const gap = page.getByRole("group", { name: "Gap" });
    await padding.getByRole("button", { name: "--space-4" }).click();
    await gap.getByRole("button", { name: "--space-2" }).click();
    expect(held.changes().map((one) => [one.property, one.token])).toEqual([
      ["padding", "--space-4"],
      ["gap", "--space-2"],
    ]);
    expect(computed(element, "padding-top")).toBe("16px");
    await expect.element(changes().getByRole("listitem").first()).toHaveTextContent("Padding");
    await expect.element(changes().getByRole("listitem").last()).toHaveTextContent("Gap");
    // The same step in the other row is not pressed.
    await expect.element(gap.getByRole("button", { name: "--space-4" })).toHaveAttribute("aria-pressed", "false");
  });

  test("Text edits the element's words, and typing them back as they were is no change", async () => {
    const element = target();
    const held = await tools(element);
    await press("Text");
    const box = page.getByRole("textbox", { name: "Text" });
    await box.fill("Fix the flaky tests");
    expect(element.textContent).toBe("Fix the flaky tests");
    expect(held.changes()).toEqual([{ kind: "text", property: "text", before: "Fix the flaky test", after: "Fix the flaky tests" }]);
    await box.fill("Fix the flaky test");
    expect(held.changes()).toEqual([]);
    expect(element.textContent).toBe("Fix the flaky test");
  });

  test("words cleared to nothing are still his to type back", async () => {
    const element = target();
    await tools(element);
    await press("Text");
    const box = page.getByRole("textbox", { name: "Text" });
    await box.fill("");
    expect(element.textContent).toBe("");
    await expect.element(page.getByRole("button", { name: "Text", exact: true })).toBeEnabled();
    await box.fill("Back");
    expect(element.textContent).toBe("Back");
  });
});

describe("the list of changes", () => {
  test("undoes one change and leaves the others on the page", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    await press("Colour");
    await press("--accent");
    await page.getByRole("button", { name: "Undo Size" }).click();
    expect(held.changes().map((one) => one.property)).toEqual(["color"]);
    expect(computed(element, "font-size")).toBe("13px");
    expect(computed(element, "color")).toBe("rgb(74, 158, 219)");
  });

  test("undoes all of them at once", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    await press("Fill");
    await press("--bg-raised");
    await press("Undo all");
    expect(held.changes()).toEqual([]);
    expect(computed(element, "font-size")).toBe("13px");
    expect(computed(element, "background-color")).toBe("rgba(0, 0, 0, 0)");
  });

  test("shows an empty value as none", async () => {
    const element = target();
    element.style.removeProperty("padding");
    await tools(element);
    await press("Spacing");
    await page.getByRole("group", { name: "Gap" }).getByRole("button", { name: "--space-2" }).click();
    await expect.element(changes().getByRole("listitem")).toHaveTextContent("→ 8px");
  });
});

describe("closing the card", () => {
  test("gives the element back as it was, every property and its words", async () => {
    const element = target();
    const held = await tools(element);
    await press("Size");
    await press("--text-lg");
    await press("Colour");
    await press("--accent");
    await press("Fill");
    await press("--bg-raised");
    await press("Text");
    await page.getByRole("textbox", { name: "Text" }).fill("Changed");
    expect(computed(element, "font-size")).toBe("20px");

    held.unmount();
    expect(computed(element, "font-size")).toBe("13px");
    expect(computed(element, "color")).toBe("rgb(255, 255, 255)");
    expect(element.style.cssText).toBe("font-size: 13px; color: rgb(255, 255, 255); padding: 0px;");
    expect(element.textContent).toBe("Fix the flaky test");
    expect(document.querySelector("[data-armada-restyled]")).toBeNull();
  });
});

describe("pictures", () => {
  test("a PNG is kept as it came, and anything that is not an image is no picture", async () => {
    const png = await asPng(new Blob([PNG], { type: "image/png" }));
    expect(new Uint8Array(png!)).toEqual(PNG);
    expect(await asPng(new Blob(["not an image"], { type: "text/plain" }))).toBeNull();
  });

  test("any other image is drawn again as a PNG", async () => {
    const canvas = document.createElement("canvas");
    canvas.width = 4;
    canvas.height = 4;
    canvas.getContext("2d")!.fillRect(0, 0, 4, 4);
    const jpeg = await new Promise<Blob>((done) => canvas.toBlob((blob) => done(blob!), "image/jpeg"));
    const png = await asPng(jpeg);
    expect(png).not.toBeNull();
    expect(isPng(new Uint8Array(png!))).toBe(true);
  });

  test("a paste or a drop yields only its image files", () => {
    const data = new DataTransfer();
    data.items.add(new File([PNG], "a.png", { type: "image/png" }));
    data.items.add(new File(["text"], "b.txt", { type: "text/plain" }));
    data.items.add(new File([PNG], "c.jpg", { type: "image/jpeg" }));
    expect(imagesIn(data).map((file) => file.name)).toEqual(["a.png", "c.jpg"]);
    expect(imagesIn(null)).toEqual([]);
  });

  test("the examples draw as thumbnails, each with a press that takes it off, and nothing where there are none", async () => {
    const removed: string[] = [];
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted.push({ root, host });
    root.render(<Examples pictures={[{ name: "x-example-1.png", png: PNG.buffer as ArrayBuffer }, { name: "x-example-2.png", png: undefined }]} onRemove={(name) => removed.push(name)} />);
    await expect.element(page.getByRole("list", { name: "Pictures" })).toBeVisible();
    await expect.element(page.getByRole("img", { name: "x-example-1.png" })).toBeInTheDocument();
    await page.getByRole("button", { name: "Remove x-example-2.png" }).click();
    expect(removed).toEqual(["x-example-2.png"]);

    root.render(<Examples pictures={[]} onRemove={() => undefined} />);
    await expect.element(page.getByRole("list", { name: "Pictures" })).not.toBeInTheDocument();
  });
});

describe("the Variant tool's tiles", () => {
  const FILE = "packages/components/src/primitives/Chip/Chip.stories.tsx";
  const index: StoryIndex = {
    Chip: { file: "packages/components/src/primitives/Chip/Chip.tsx", title: "Chip", stories: [{ name: "Disabled", args: {}, file: FILE }, { name: "Loud", args: { tone: "loud" }, file: FILE }] },
  };
  const module = { default: { component: ({ tone }: { tone?: string }) => <i data-drawn={tone ?? "plain"}>drawn</i> }, Disabled: {}, Loud: { args: { tone: "loud" } } } as unknown as StoryModule;

  async function variants(story: Sink["story"]): Promise<void> {
    theme();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted.push({ root, host });
    root.render(<Tools id="20260916-222105-k3f9" element={target()} changes={[]} onChanges={() => undefined} keep="test:text" names={["Chip"]} stories={index} story={story} />);
    await press("Variant");
    await expect.element(page.getByRole("group", { name: "Variant" })).toBeVisible();
  }

  test("draw each story aria-hidden and out of reach, and keep the story's name as the button's", async () => {
    await variants(async (file) => (file === FILE ? module : null));
    const disabled = page.getByRole("button", { name: "Disabled", exact: true });
    await expect.element(disabled).toBeVisible();
    const button = disabled.element();
    const preview = button.querySelector<HTMLElement>(".armada-annotate__preview")!;
    expect(preview.getAttribute("aria-hidden")).toBe("true");
    expect(preview.hasAttribute("inert")).toBe(true);
    await expect.poll(() => preview.querySelector("[data-drawn]")?.getAttribute("data-drawn")).toBe("plain");
    expect(button.querySelector('.armada-annotate__preview [data-drawn="loud"]')).toBeNull();
    expect(page.getByRole("button", { name: "Loud", exact: true }).element().querySelector('[data-drawn="loud"]')).not.toBeNull();
  });

  test("are the names alone without a loader", async () => {
    await variants(undefined);
    const disabled = page.getByRole("button", { name: "Disabled", exact: true });
    await expect.element(disabled).toBeVisible();
    expect(disabled.element().querySelector(".armada-annotate__preview")).toBeNull();
    expect(disabled.element().textContent).toBe("Disabled");
  });

  test("are the names alone where the file is unknown", async () => {
    await variants(async () => null);
    const disabled = page.getByRole("button", { name: "Disabled", exact: true });
    await expect.element(disabled).toBeVisible();
    await expect.poll(() => disabled.element().querySelector(".armada-annotate__preview")).toBeNull();
  });

  const MANY = Array.from({ length: 20 }, (_, at) => `Story${at}`);
  const many: StoryIndex = { Chip: { file: index.Chip!.file, title: "Chip", stories: MANY.map((name) => ({ name, args: {}, file: FILE })) } };
  const manyModule = {
    default: { component: () => <i data-drawn="">drawn</i> },
    ...Object.fromEntries(MANY.map((name) => [name, {}])),
  } as unknown as StoryModule;

  /** The tools as the layer draws them: in a card capped at `max` pixels, a line over them and the card's acts under them. */
  async function inACard(max: number): Promise<{ card: HTMLElement; tiles: HTMLElement; over: HTMLElement; save: HTMLElement }> {
    theme();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted.push({ root, host });
    const style = { "--annotate-card-x": "0px", "--annotate-card-y": "0px", "--annotate-card-max": `${max}px` } as CSSProperties;
    root.render(
      <div className="armada-annotate">
        <div className="armada-annotate__card" data-card="" style={style}>
          <div className="armada-annotate__body">
            <p data-over="">The note</p>
            <Tools id="20260916-222105-k3f9" element={target()} changes={[]} onChanges={() => undefined} keep="test:text" names={["Chip"]} stories={many} story={async () => manyModule} />
            <div className="armada-annotate__actions">
              <button type="button" data-save="">
                Save note
              </button>
            </div>
          </div>
        </div>
      </div>,
    );
    await press("Variant");
    await expect.poll(() => host.querySelector("[data-tiles] [data-drawn]")).not.toBeNull();
    const one = <T extends HTMLElement>(selector: string): T => host.querySelector<T>(selector)!;
    return { card: one("[data-card]"), tiles: one("[data-tiles]"), over: one("[data-over]"), save: one("[data-save]") };
  }

  test("scroll alone in a capped card, and what is over and under them stays put", async () => {
    const { card, tiles, over, save } = await inACard(480);
    expect(tiles.scrollHeight).toBeGreaterThan(tiles.clientHeight);
    expect(card.scrollHeight).toBe(card.clientHeight);
    const wasOver = over.getBoundingClientRect().top;
    const wasSave = save.getBoundingClientRect().top;
    expect(save.getBoundingClientRect().bottom).toBeLessThanOrEqual(card.getBoundingClientRect().bottom);

    tiles.scrollTop = 200;
    expect(tiles.scrollTop).toBe(200);
    expect(over.getBoundingClientRect().top).toBe(wasOver);
    expect(save.getBoundingClientRect().top).toBe(wasSave);
    expect(card.scrollTop).toBe(0);
  });

  test("hand the scroll back to a card too short to hold them, and the acts stay in reach", async () => {
    const { card, tiles, save } = await inACard(180);
    expect(card.scrollHeight).toBeGreaterThan(card.clientHeight);
    expect(tiles.clientHeight).toBeGreaterThan(0);
    expect(save.getBoundingClientRect().bottom).toBeLessThanOrEqual(card.getBoundingClientRect().bottom);
  });

  test("leave a drawn story's own button as it draws", async () => {
    const own = { default: {}, Disabled: { render: () => <button type="button" className="armada-button" data-own="" /> }, Loud: { render: () => <i /> } } as unknown as StoryModule;
    theme();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted.push({ root, host });
    root.render(
      <div className="armada-annotate">
        <div className="armada-annotate__card">
          <Tools id="20260916-222105-k3f9" element={target()} changes={[]} onChanges={() => undefined} keep="test:text" names={["Chip"]} stories={index} story={async () => own} />
        </div>
      </div>,
    );
    await press("Variant");
    await expect.poll(() => host.querySelector("[data-own]")).not.toBeNull();
    const tile = page.getByRole("button", { name: "Disabled", exact: true }).element();
    expect(getComputedStyle(tile).flexDirection).toBe("column");
    expect(getComputedStyle(host.querySelector("[data-own]")!).flexDirection).not.toBe("column");
  });
});
