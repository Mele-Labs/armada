import type { Meta, StoryObj } from "@storybook/react-vite";
import { PowerOff } from "lucide-react";
import { expect, fn, screen, waitFor, within } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { SessionFrame } from "./SessionFrame";

/** A Job open: a band in the state's hue over its body. The state is the frame's hue and one mark. */
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
    await expect(canvas.getByRole("img", { name: /^Blank: no slot, no branch/ })).toBeInTheDocument();
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
    const field = canvas.getByRole("textbox", { name: "Job name" });
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
    await userEvent.type(canvas.getByRole("textbox", { name: "Job name" }), "Fresh name{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("Fresh name");
  },
};

/** Held until the plan's limit resets: one mark in place of the live one, saying when it resumes. */
export const Limited: Story = {
  args: { state: "limited", said: "Limit reached · resumes 6:40 PM", title: "Invoice PDF layout" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: /^Limit reached · resumes 6:40 PM/ })).toBeInTheDocument();
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
    await expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    await userEvent.hover(figure);
    const card = await screen.findByRole("tooltip");
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
    await expect(screen.getByRole("dialog", { name: "Context" })).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
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
 * The head is one row. The state glyph leads it and its tooltip carries the address; the model is not drawn here (the
 * composer's Model select reads it). The title keeps to one line, the whole of it in its tooltip, and nothing overlaps.
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
      <Button variant="ghost" size="sm" aria-label="End job">
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
    const glyph = canvas.getByRole("img", { name: "Working · s-e07cfe09" });
    await expect(canvas.queryByRole("img", { name: /^Model/ })).not.toBeInTheDocument();

    const title = canvas.getByRole("button", { name: `${LONG}, rename` });
    await expect(title).toHaveAccessibleDescription(LONG);
    const line = parseFloat(getComputedStyle(title).lineHeight);
    await expect(title.getBoundingClientRect().height).toBeLessThan(line * 1.5);
    await expect(title.scrollWidth).toBeGreaterThan(title.clientWidth);

    // One row: the state glyph, the title, the act and the figure share a centre line.
    const centre = (one: Element) => {
      const box = one.getBoundingClientRect();
      return box.top + box.height / 2;
    };
    const row = [glyph, title, canvas.getByRole("button", { name: "End job" }), canvas.getByRole("button", { name: "Context 5%" })];
    for (const one of row) await expect(Math.abs(centre(one) - centre(glyph))).toBeLessThan(8);
  },
};

/** Big or old: New Job and Fork are items in the More menu, and the head draws no mark of its own for them. */
export const FreshStart: Story = {
  args: {
    state: "idle",
    said: "Idle",
    title: "Cart totals rounding",
    context: { tokens: 612_000, window: 1_000_000 },
    fresh: { tokens: 612_000, hours: 52, onFresh: fn(), onFork: fn() },
    menu: { entries: [{ kind: "item", id: "retro", label: "Retro" }], onSelect: fn() },
  },
  play: async ({ canvas, userEvent, args }) => {
    await expect(canvas.queryByRole("button", { name: "Fresh start" })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    const menu = await screen.findByRole("menu");
    await expect(within(menu).getAllByRole("menuitem").map((one) => one.textContent)).toEqual([expect.stringContaining("Retro"), expect.stringContaining("New Job"), expect.stringContaining("Fork")]);

    await userEvent.click(within(menu).getByRole("menuitem", { name: /^New Job/ }));
    await expect(args.fresh?.onFresh).toHaveBeenCalledOnce();
    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    await userEvent.click(within(await screen.findByRole("menu")).getByRole("menuitem", { name: "Fork" }));
    await expect(args.fresh?.onFork).toHaveBeenCalledOnce();
    await expect(args.menu?.onSelect).not.toHaveBeenCalled();

    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    await userEvent.click(within(await screen.findByRole("menu")).getByRole("menuitem", { name: "Retro" }));
    await expect(args.menu?.onSelect).toHaveBeenCalledWith("retro");
  },
};

/** While the old Session writes its handoff New Job is listed and waits, and Fork is left offered. */
export const FreshStartWaiting: Story = {
  args: {
    state: "idle",
    said: "Idle",
    title: "Cart totals rounding",
    fresh: { tokens: 612_000, hours: 3, busy: true, onFresh: fn(), onFork: fn() },
  },
  play: async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    const menu = await screen.findByRole("menu");
    const wait = within(menu).getByRole("menuitem", { name: /^New Job/ });
    await expect(wait).toHaveAttribute("aria-disabled", "true");
    wait.focus();
    await userEvent.keyboard("{Enter}");
    await expect(args.fresh?.onFresh).not.toHaveBeenCalled();
    await expect(within(menu).getByRole("menuitem", { name: "Fork" })).not.toHaveAttribute("aria-disabled", "true");
  },
};

/** A Fleet with no fork of a live Session offers none. */
export const FreshStartWithoutFork: Story = {
  args: { state: "idle", said: "Idle", title: "Cart totals rounding", fresh: { tokens: 612_000, hours: 3, onFresh: fn() } },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    const menu = await screen.findByRole("menu");
    await expect(within(menu).getByRole("menuitem", { name: /^New Job/ })).not.toHaveAttribute("aria-disabled", "true");
    await expect(within(menu).queryByRole("menuitem", { name: "Fork" })).not.toBeInTheDocument();
  },
};
