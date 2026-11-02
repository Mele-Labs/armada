// A story module drawn without Storybook: a render, a component with merged args, decorators, and a story that throws.

import { useState, type ComponentType } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { StoryBoundary, storyElement, type StoryModule } from "./story";

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
});

/** The story drawn on the page. A caught throw is the boundary's doing and is not logged. */
function draw(module: StoryModule, name: string): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host, { onCaughtError: () => undefined });
  root.render(<StoryBoundary>{storyElement(module, name)}</StoryBoundary>);
  mounted.push({ root, host });
  return host;
}

const Label = ({ text, tone }: { text?: string; tone?: string }) => (
  <p data-tone={tone}>{text}</p>
);

describe("a story", () => {
  test("with a render is drawn by it, given its args", async () => {
    const module = { default: { component: Label, args: { text: "meta" } }, Loud: { args: { text: "story" }, render: (args: { text: string }) => <h2>{args.text} from render</h2> } } as unknown as StoryModule;
    draw(module, "Loud");
    await expect.element(page.getByRole("heading", { name: "story from render" })).toBeVisible();
  });

  test("with only the component draws it with the meta's args and the story's over them", async () => {
    const module = { default: { component: Label, args: { text: "meta", tone: "calm" } }, Loud: { args: { tone: "loud" } } } as unknown as StoryModule;
    const host = draw(module, "Loud");
    await expect.element(page.getByText("meta")).toBeVisible();
    expect(host.querySelector("p")!.getAttribute("data-tone")).toBe("loud");
  });

  test("uses hooks in its render", async () => {
    const module = {
      default: {},
      Counted: {
        render: () => {
          const [count] = useState(4);
          return <p>count {count}</p>;
        },
      },
    } as unknown as StoryModule;
    draw(module, "Counted");
    await expect.element(page.getByText("count 4")).toBeVisible();
  });

  test("is wrapped by its decorators, then the meta's", async () => {
    const around = (tag: string): ((Story: ComponentType) => React.ReactNode) => (Story) => (
      <div data-around={tag}>
        <Story />
      </div>
    );
    const module = { default: { component: Label, args: { text: "inside" }, decorators: [around("meta")] }, Wrapped: { decorators: [around("story")] } } as unknown as StoryModule;
    const host = draw(module, "Wrapped");
    await expect.element(page.getByText("inside")).toBeVisible();
    expect(host.querySelector('[data-around="meta"] > [data-around="story"] p')).not.toBeNull();
  });

  test("that throws draws nothing, and the boundary catches it", async () => {
    const module = {
      default: { component: Label },
      Broken: {
        render: () => {
          throw new Error("no story here");
        },
      },
    } as unknown as StoryModule;
    const host = draw(module, "Broken");
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(host.innerHTML).toBe("");
  });

  test("that is not in the module, or has nothing to draw it with, is no element", () => {
    const module = { default: {}, Bare: {} } as unknown as StoryModule;
    expect(storyElement(module, "Missing")).toBeNull();
    expect(storyElement(module, "Bare")).toBeNull();
  });
});
