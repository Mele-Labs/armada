import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { colourCode } from "./colourCode";

/**
 * A fenced block's words in spans classed by role, with the hues left to the stylesheet that draws the block. A
 * language it does not know stays the plain words.
 */
function Block({ language, source }: { language: string | undefined; source: string }) {
  return <pre aria-label="Code">{colourCode(language, source)}</pre>;
}

const meta: Meta<typeof Block> = {
  title: "Compositions/Session thread/Code colour",
  component: Block,
};
export default meta;

type Story = StoryObj<typeof Block>;

/** A keyword and a literal each take their role's class, and the words read the same as the source. */
export const KnownLanguage: Story = {
  args: {
    language: "ts",
    source: "const a = 1",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("const")).toHaveClass("tok-keyword");
    await expect(canvas.getByText("1")).toHaveClass("tok-literal");
    await expect(canvas.getByLabelText("Code")).toHaveTextContent("const a = 1");
  },
};

/** The language is read in either case. */
export const LanguageInCapitals: Story = {
  args: {
    language: "TS",
    source: "const a = 1",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("const")).toHaveClass("tok-keyword");
  },
};

/** A language it has no grammar for comes back as the source, with no span in it. */
export const UnknownLanguage: Story = {
  args: {
    language: "brainfuck",
    source: "const a = 1",
  },
  play: async ({ canvas }) => {
    const block = canvas.getByLabelText("Code");
    await expect(block.textContent).toBe("const a = 1");
    await expect(block.querySelector("span")).toBeNull();
  },
};

/** A fence with no language is plain too. */
export const NoLanguage: Story = {
  args: {
    language: undefined,
    source: "const a = 1",
  },
  play: async ({ canvas }) => {
    const block = canvas.getByLabelText("Code");
    await expect(block.textContent).toBe("const a = 1");
    await expect(block.querySelector("span")).toBeNull();
  },
};
