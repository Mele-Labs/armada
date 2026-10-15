import type { Meta, StoryObj } from "@storybook/react-vite";
import { PowerOff } from "lucide-react";
import { expect, fn, within } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { SessionFrame } from "./SessionFrame";

/** A Session open: a band in the state's hue over its body. The state is the frame's hue and one mark. */
const meta: Meta<typeof SessionFrame> = {
  title: "Compositions/Session frame",
  component: SessionFrame,
  args: { id: "s7", children: <div style={{ height: "calc(var(--space-12) * 4)" }} /> },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", height: "calc(var(--space-12) * 5)", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionFrame>;

/** No slot, no branch, no title yet. */
export const Blank: Story = {
  args: { state: "blank", said: "Blank: no slot, no branch" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Blank: no slot, no branch" })).toBeInTheDocument();
  },
};

/** A turn is running; the mark breathes. */
export const Working: Story = { args: { state: "working", said: "Working", title: "Flaky store test" } };

/** Idle and the person's move. */
export const Waiting: Story = { args: { state: "waiting", said: "Waiting on you", title: "Flaky store test" } };

/** A pull request's Checks are red. */
export const Failing: Story = { args: { state: "failing", said: "Checks failed on #1843", title: "Flaky store test" } };

/** The title is edited where it stands: a press opens it, Enter saves it and Esc keeps the old one. */
export const Renamable: Story = {
  args: { state: "waiting", said: "Waiting on you", title: "Flaky store test", onRename: fn() },
  play: async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Flaky store test, rename" }));
    await userEvent.keyboard("{Escape}");
    await expect(args.onRename).not.toHaveBeenCalled();
    await expect(canvas.getByRole("button", { name: "Flaky store test, rename" })).toBeInTheDocument();

    await userEvent.click(canvas.getByRole("button", { name: "Flaky store test, rename" }));
    const field = canvas.getByRole("textbox", { name: "Session name" });
    await userEvent.clear(field);
    await userEvent.type(field, "Store test, flaky on CI{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("Store test, flaky on CI");
  },
};

/** A terminal Session whose mod is older than the repository's is marked in its header. */
export const ModOutOfDate: Story = {
  args: { state: "waiting", said: "Waiting on you", title: "Release notes script", modOutOfDate: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Mod out of date: run /reload-plugins" })).toBeInTheDocument();
  },
};

/** No title yet: the header offers Rename as an icon, since there is no name to press. */
export const Untitled: Story = {
  args: { state: "waiting", said: "Waiting on you", onRename: fn() },
  play: async ({ canvas, userEvent, args }) => {
    await expect(canvas.getByRole("button", { name: "Rename" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Rename" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Session name" }), "Fresh name{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("Fresh name");
  },
};

/** Held until the plan's limit resets: one mark in place of the live one, saying when it resumes. */
export const Limited: Story = {
  args: { state: "limited", said: "Limit reached · resumes 6:40 PM", title: "Invoice PDF layout" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Limit reached · resumes 6:40 PM" })).toBeInTheDocument();
  },
};

const breakdown = {
  categories: [
    { name: "System prompt", tokens: 3_100, kind: "used" as const },
    { name: "Messages", tokens: 50_740, kind: "used" as const },
    { name: "Free space", tokens: 913_160, kind: "free" as const },
    { name: "Autocompact buffer", tokens: 33_000, kind: "buffer" as const },
  ],
  memoryFiles: [
    { path: "NOTES.md", tokens: 2_100 },
    { path: "apps/NOTES.md", tokens: 700 },
  ],
  mcpTools: [
    { name: "list_jobs", server: "armada-fleet", tokens: 2_600 },
    { name: "get_job", server: "armada-fleet", tokens: 2_400 },
    { name: "query", server: "gitnexus", tokens: 1_500 },
  ],
  totalTokens: 53_840,
  maxTokens: 1_000_000,
};

/** The context tank: the live tokens and the percent of the window. With a breakdown it opens a card of what fills it, and a press keeps it open. */
export const ContextBreakdown: Story = {
  args: { state: "idle", said: "Idle", title: "Release notes script", context: { tokens: 53_840, window: 1_000_000, breakdown } },
  play: async ({ canvas, userEvent }) => {
    const figure = canvas.getByRole("button", { name: "Context 5%" });
    await expect(figure).toHaveTextContent("53.8k · 5%");
    await expect(canvas.queryByRole("dialog")).not.toBeInTheDocument();

    await userEvent.hover(figure);
    const card = canvas.getByRole("dialog", { name: "Context" });
    await expect(card).toHaveTextContent("5% used");
    await expect(card).toHaveTextContent("53.8k / 1M");
    await expect(within(card).getByText("System prompt")).toBeVisible();
    await expect(within(card).getByText("913k")).toBeVisible();
    // The heaviest servers, their tools summed.
    await expect(within(card).getByText("armada-fleet")).toBeVisible();
    await expect(within(card).getByText("5.0k")).toBeVisible();
    await expect(within(card).getByText("apps/NOTES.md")).toBeVisible();

    await userEvent.click(figure);
    await userEvent.unhover(figure);
    await expect(canvas.getByRole("dialog", { name: "Context" })).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).not.toBeInTheDocument();
  },
};

/** Without a breakdown the tank is a figure and not a button, and says its tokens on hover. */
export const ContextWithoutBreakdown: Story = {
  args: { state: "idle", said: "Idle", title: "Release notes script", context: { tokens: 142_000, window: 200_000 } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Context 71%" })).toHaveTextContent("142k · 71%");
    await expect(canvas.queryByRole("button", { name: /^Context/ })).not.toBeInTheDocument();
  },
};

const LONG = "How does armada interface with GitHub (PRs, Issues, Projects)";

/**
 * The head is one row. The address is in the terminal glyph's tooltip and the model is not drawn here (the composer's
 * Model select reads it). The title keeps to one line, the whole of it in its tooltip, and nothing overlaps.
 */
export const HeadOneRow: Story = {
  args: {
    state: "working",
    said: "Working",
    id: "e07cfe09-5b21",
    address: "s-e07cfe09",
    title: LONG,
    onRename: fn(),
    context: { tokens: 48_400, window: 1_000_000, breakdown },
    actions: (
      <Button variant="ghost" size="sm" aria-label="End session">
        <PowerOff size={16} strokeWidth={2} aria-hidden />
      </Button>
    ),
  },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", width: "calc(var(--space-12) * 9)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas }) => {
    const glyph = canvas.getByRole("img", { name: "s-e07cfe09" });
    await expect(glyph.parentElement).toHaveAccessibleDescription("s-e07cfe09");
    await expect(canvas.queryByRole("img", { name: /^Model/ })).not.toBeInTheDocument();

    const title = canvas.getByRole("button", { name: `${LONG}, rename` });
    await expect(title).toHaveAccessibleDescription(LONG);
    const line = parseFloat(getComputedStyle(title).lineHeight);
    await expect(title.getBoundingClientRect().height).toBeLessThan(line * 1.5);
    await expect(title.scrollWidth).toBeGreaterThan(title.clientWidth);

    // One row: the glyph, the title, the act, the figure and the state mark share a centre line.
    const centre = (one: Element) => {
      const box = one.getBoundingClientRect();
      return box.top + box.height / 2;
    };
    const row = [glyph, title, canvas.getByRole("button", { name: "End session" }), canvas.getByRole("button", { name: "Context 5%" }), canvas.getByRole("img", { name: "Working" })];
    for (const one of row) await expect(Math.abs(centre(one) - centre(glyph))).toBeLessThan(8);
  },
};

/** Big or old: a quiet mark after the context tank. Hover opens its card, a press keeps it open, and each act is its own button. */
export const FreshStart: Story = {
  args: {
    state: "idle",
    said: "Idle",
    title: "Cart totals rounding",
    context: { tokens: 612_000, window: 1_000_000 },
    fresh: { tokens: 612_000, hours: 52, onFresh: fn(), onFork: fn() },
  },
  play: async ({ canvas, userEvent, args }) => {
    const mark = canvas.getByRole("button", { name: "Fresh start" });
    await expect(canvas.queryByRole("dialog")).not.toBeInTheDocument();

    await userEvent.hover(mark);
    const card = canvas.getByRole("dialog", { name: "Fresh start" });
    await expect(card).toHaveTextContent("612k context");
    await expect(card).toHaveTextContent("Started 2d ago");

    await userEvent.click(mark);
    await userEvent.unhover(mark);
    await expect(canvas.getByRole("dialog", { name: "Fresh start" })).toBeVisible();

    await userEvent.click(within(card).getByRole("button", { name: "New Session" }));
    await expect(args.fresh?.onFresh).toHaveBeenCalledOnce();
    await userEvent.click(within(card).getByRole("button", { name: "Fork" }));
    await expect(args.fresh?.onFork).toHaveBeenCalledOnce();

    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).not.toBeInTheDocument();
  },
};

/** While the old Session writes its handoff New Session waits, and Fork is left offered. A Fleet with no fork of a live Session offers none. */
export const FreshStartWaiting: Story = {
  args: {
    state: "idle",
    said: "Idle",
    title: "Cart totals rounding",
    fresh: { tokens: 612_000, hours: 3, busy: true, onFresh: fn(), onFork: fn() },
  },
  play: async ({ canvas, userEvent }) => {
    const mark = canvas.getByRole("button", { name: "Fresh start" });
    await expect(mark).toHaveAttribute("aria-busy", "true");
    await userEvent.click(mark);
    await expect(within(canvas.getByRole("dialog", { name: "Fresh start" })).getByRole("button", { name: "New Session" })).toBeDisabled();
  },
};

export const FreshStartWithoutFork: Story = {
  args: { state: "idle", said: "Idle", title: "Cart totals rounding", fresh: { tokens: 612_000, hours: 3, onFresh: fn() } },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Fresh start" }));
    const card = canvas.getByRole("dialog", { name: "Fresh start" });
    await expect(within(card).getByRole("button", { name: "New Session" })).toBeEnabled();
    await expect(within(card).queryByRole("button", { name: "Fork" })).not.toBeInTheDocument();
  },
};
