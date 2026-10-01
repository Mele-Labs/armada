import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Prose, ProseLinks } from "./Prose";

/** What a click hands over. Bridge's is `window.armada.openLink`. */
const opened = fn();

const meta: Meta<typeof Prose> = {
  title: "Primitives/Prose",
  component: Prose,
  decorators: [
    (Story) => (
      <ProseLinks.Provider value={opened}>
        <Story />
      </ProseLinks.Provider>
    ),
  ],
  beforeEach: () => {
    opened.mockClear();
  },
};
export default meta;

type Story = StoryObj<typeof Prose>;

/**
 * A Judge's `consequence`: several paragraphs with a path and an expression
 * in them. Rendered as one string, this was the wall the override dialog was
 * reported for.
 */
export const AJudgesConsequence: Story = {
  args: {
    text:
      "The route table is walked once per operation and the loop skips `forget_job`, so the " +
      "assertion that every operation is served passes without ever reading the one operation " +
      "the step was about.\n\n" +
      "The skip is in `crates/api/src/tests/served.rs` and reads:\n\n" +
      "```rust\nif route.operation == \"forget_job\" { continue; }\n```\n\n" +
      "Nothing else in the file narrows the set, so the count the assertion compares against " +
      "was lowered by the same edit that made it pass.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("paragraph")).toHaveLength(3);
    await expect(canvas.getByText("crates/api/src/tests/served.rs")).toBeVisible();
    await expect(
      canvas.getByText('if route.operation == "forget_job" { continue; }'),
    ).toBeVisible();
    await expect(canvas.queryByText(/`/)).toBeNull();
  },
};

/**
 * A citation on a gaming flag. Inline code stays inline, and the fenced block
 * wraps rather than clipping, because the failing render broke an expression
 * mid-token.
 */
export const AFlagsCitation: Story = {
  args: {
    text:
      "`crates/api/src/tests/served.rs:214` — the `served_every_operation` assertion counts " +
      "`ROUTES.len()` after the filter rather than before it:\n\n" +
      "```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != \"forget_job\").collect();\n" +
      "assert_eq!(routes.len(), served.len());\n```",
  },
};

/**
 * The structure a model writes when it lists findings. The `#` line renders at
 * `--weight-heading` and full contrast, **not at `--text-lg`**: panel headings
 * own that step. It takes no place in the page's outline either, since a
 * model's `#` inside a card is not the card's title.
 */
export const AListAndAHeading: Story = {
  args: {
    text:
      "# What the check read\n\n" +
      "Three things, and the third is the one that matters:\n\n" +
      "- the diff touches `armada.yml`\n" +
      "- the touched key is `checks.tests.command`\n" +
      "- the command it was changed **to** exits 0 on an empty test set\n\n" +
      "The first two alone are ordinary. Together with the third they are the pattern.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("What the check read")).toBeVisible();
    await expect(canvas.queryByText(/#/)).toBeNull();
    await expect(canvas.queryByRole("heading")).toBeNull();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(3);
  },
};

/** Strong is contrast, emphasis is slant, and a strikethrough is a line through. */
export const Emphasis: Story = {
  args: {
    text: "The command was changed **to** exit 0, *after* the review, and ~~before~~ the Judge read it.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("strong")).toHaveTextContent("to");
    await expect(canvas.getByRole("emphasis")).toHaveTextContent("after");
    await expect(canvas.getByRole("deletion")).toHaveTextContent("before");
    await expect(canvas.queryByText(/[*~]/)).toBeNull();
  },
};

/** An ordered list keeps its numbers, because they carry the order. */
export const AnOrderedList: Story = {
  args: {
    text:
      "1. Read the diff\n" +
      "2. Run the touched Check\n" +
      "3. Compare the count against `main`\n" +
      "   - before the filter\n" +
      "   - after it",
  },
  play: async ({ canvas }) => {
    const [steps, nested] = canvas.getAllByRole("list");
    await expect(steps?.tagName).toBe("OL");
    await expect(nested?.tagName).toBe("UL");
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
  },
};

/** A task list's state is a disabled checkbox, so it is read as well as seen. */
export const ATaskList: Story = {
  args: {
    text: "- [x] Reproduce the skip\n- [ ] Restore `forget_job` to the walk",
  },
  play: async ({ canvas }) => {
    const [done, open] = canvas.getAllByRole("checkbox");
    await expect(done).toBeChecked();
    await expect(open).not.toBeChecked();
    await expect(done).toBeDisabled();
  },
};

/**
 * A table drawn with the Table primitive's rules and header contrast, at the
 * surrounding size. A cell wraps rather than scrolling.
 */
export const ATable: Story = {
  args: {
    text:
      "| Check | Before | After |\n" +
      "|---|---:|---:|\n" +
      "| `served_every_operation` | 41 | 40 |\n" +
      "| `routes_are_documented` | 41 | 41 |",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("table")).toBeVisible();
    await expect(canvas.getAllByRole("columnheader").map((cell) => cell.textContent)).toEqual([
      "Check",
      "Before",
      "After",
    ]);
    await expect(canvas.getAllByRole("row")).toHaveLength(3);
  },
};

/** A quote and a rule. Neither keeps its characters. */
export const AQuoteAndARule: Story = {
  args: {
    text:
      "The brief said:\n\n" +
      "> Every operation in `operations.toml` is served by exactly one route.\n\n" +
      "---\n\n" +
      "The assertion checked one fewer.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("blockquote")).toHaveTextContent(
      "Every operation in operations.toml is served by exactly one route.",
    );
    await expect(canvas.queryByText(/^>|---/)).toBeNull();
  },
};

/**
 * Links, in `--accent`. A click hands the address to `ProseLinks` and the
 * window goes nowhere. Only `http(s):` is a link. A `javascript:` address and
 * a relative path draw as their text, and an image is a link to it rather than
 * a fetch.
 */
export const Links: Story = {
  args: {
    text:
      "The upstream fix is [tokio#7012](https://github.com/tokio-rs/tokio/pull/7012), and " +
      "https://docs.rs/tokio says the same.\n\n" +
      "[This one](javascript:alert(1)) and [this one](../served.rs) are not links.\n\n" +
      "![the failing render](https://example.com/render.png)",
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("link", { name: "tokio#7012" }));
    await expect(opened).toHaveBeenCalledWith("https://github.com/tokio-rs/tokio/pull/7012");
    await expect(canvas.getByRole("link", { name: "https://docs.rs/tokio" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "the failing render" })).toBeVisible();
    await expect(canvas.queryByRole("img")).toBeNull();
    await expect(canvas.queryByRole("link", { name: "This one" })).toBeNull();
    await expect(canvas.queryByRole("link", { name: "this one" })).toBeNull();
    await expect(canvas.getByText("This one and this one are not links.")).toBeVisible();
    await expect(opened).toHaveBeenCalledTimes(1);
  },
};

/** With nothing to open it, a link is its text: no control where there is nowhere to go. */
export const LinksWithNowhereToOpen: Story = {
  args: {
    text: "The upstream fix is [tokio#7012](https://github.com/tokio-rs/tokio/pull/7012).",
  },
  decorators: [
    (Story) => (
      <ProseLinks.Provider value={null}>
        <Story />
      </ProseLinks.Provider>
    ),
  ],
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("link")).toBeNull();
    await expect(canvas.getByText(/tokio#7012/)).toBeVisible();
  },
};

/** Raw HTML is the characters it was written as. This text arrives from a model. */
export const RawHtml: Story = {
  args: {
    text: 'A tag is written <b>like this</b> and stays that way.\n\n<img src="x" onerror="alert(1)">',
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText("A tag is written <b>like this</b> and stays that way."),
    ).toBeVisible();
    await expect(canvas.getByText('<img src="x" onerror="alert(1)">')).toBeVisible();
    await expect(canvas.queryByRole("strong")).toBeNull();
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};

/** Empty text draws nothing rather than an empty block. */
export const Empty: Story = {
  args: { text: "  \n\n" },
  play: async ({ canvasElement }) => {
    await expect(canvasElement).toBeEmptyDOMElement();
  },
};
