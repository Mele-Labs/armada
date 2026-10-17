import type { Meta, StoryObj } from "@storybook/react-vite";
import { Box, GitPullRequest } from "lucide-react";
import { expect, fn, screen, waitFor } from "storybook/test";

import { Inspect, InspectCard } from "./Inspect";

/**
 * The one card every rich hover opens. `InspectCard` is the surface; `Inspect` hangs it off an element and owns when it
 * opens, pins and closes. The plain `Tooltip` stays for one line of words.
 */
const meta: Meta<typeof Inspect> = {
  title: "Primitives/Inspect",
  component: Inspect,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-12)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof Inspect>;

const FACTS = [
  { label: "Bay", value: "slot-3", mono: true },
  { label: "Step", value: "2 of 5, Build" },
  { label: "Last", value: "4m", mono: true },
];

const card = (
  <InspectCard
    kind={{ Glyph: Box, label: "Job" }}
    where="armada"
    title="Cache the manifest read"
    state={{ said: "Working", tone: "var(--status-running)" }}
    facts={FACTS}
    keys={[{ key: "Enter", verb: "Open" }]}
  />
);

/** Sets a timing token on the root for one story and puts it back, so the delay under test is the one written here. */
function timing(delay: string, grace = "0ms"): () => void {
  const root = document.documentElement.style;
  const was = [root.getPropertyValue("--tooltip-delay"), root.getPropertyValue("--tooltip-grace")];
  root.setProperty("--tooltip-delay", delay);
  root.setProperty("--tooltip-grace", grace);
  return () => {
    root.setProperty("--tooltip-delay", was[0] ?? "");
    root.setProperty("--tooltip-grace", was[1] ?? "");
  };
}

const wait = (ms: number) => new Promise((done) => window.setTimeout(done, ms));

/** The surface alone: kind head, name, state band, facts and the keys footer ending in Esc. */
export const Anatomy: StoryObj<typeof InspectCard> = {
  render: () => (
    <InspectCard
      kind={{ Glyph: GitPullRequest, label: "Pull request" }}
      where="#1890"
      title="Open the inspect card on every row"
      state={{ said: "Checks running", tone: "var(--status-running)" }}
      facts={[...FACTS, { label: "Branch", value: "style/inspect-card", mono: true }]}
      keys={[{ key: "Enter", verb: "Open" }]}
    />
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Pull request")).toBeVisible();
    await expect(canvas.getByText("#1890")).toBeVisible();
    await expect(canvas.getByText("Open the inspect card on every row")).toBeVisible();
    await expect(canvas.getByText("Checks running")).toBeVisible();
    const keys = canvas.getByRole("list", { name: "Keys" });
    const items = keys.querySelectorAll("li");
    await expect(items.length).toBe(2);
    await expect(items[0]).toHaveTextContent("Open");
    await expect(items[1]).toHaveTextContent(/Esc/);
  },
};

/** Every fact's value is on the right edge of the card, in tabular figures, so a column of figures lines up. */
export const FactsAreRightAligned: StoryObj<typeof InspectCard> = {
  render: () => <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Facts" facts={FACTS} />,
  play: async ({ canvas }) => {
    for (const value of ["slot-3", "2 of 5, Build", "4m"]) {
      const cell = canvas.getByText(value);
      await expect(getComputedStyle(cell).textAlign).toMatch(/^(right|end)$/);
      await expect(getComputedStyle(cell).fontVariantNumeric).toContain("tabular-nums");
    }
    const [short, long] = [canvas.getByText("4m"), canvas.getByText("2 of 5, Build")];
    await expect(Math.round(short.getBoundingClientRect().right)).toBe(Math.round(long.getBoundingClientRect().right));
  },
};

/** A hover opens it only after `--tooltip-delay`, and a plain hover is a tooltip, not yet a dialog. */
export const OpensAfterTheDelay: Story = {
  render: () => (
    <Inspect card={card} name="Job">
      <button type="button">Trigger</button>
    </Inspect>
  ),
  play: async ({ canvas, userEvent }) => {
    const undo = timing("250ms");
    try {
      await userEvent.hover(canvas.getByRole("button", { name: "Trigger" }));
      await expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
      await waitFor(() => expect(screen.getByRole("tooltip")).toBeVisible());
      await expect(screen.getByRole("tooltip")).toHaveTextContent("Cache the manifest read");
      await expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      await userEvent.unhover(canvas.getByRole("button", { name: "Trigger" }));
      await waitFor(() => expect(screen.queryByRole("tooltip")).not.toBeInTheDocument());
    } finally {
      undo();
    }
  },
};

/** Keyboard focus opens it too, after the same delay, and the trigger is described by it. */
export const OpensOnKeyboardFocus: Story = {
  render: () => (
    <Inspect card={card} name="Job">
      <button type="button">Trigger</button>
    </Inspect>
  ),
  play: async ({ canvas, userEvent }) => {
    const undo = timing("150ms");
    try {
      await userEvent.tab();
      const trigger = canvas.getByRole("button", { name: "Trigger" });
      await expect(trigger).toHaveFocus();
      await waitFor(() => expect(screen.getByRole("tooltip")).toBeVisible());
      await expect(trigger).toHaveAccessibleDescription(/Cache the manifest read/);
    } finally {
      undo();
    }
  },
};

/** Moving from one open card to the next item opens that one at once, with no delay, and the first is put away. */
export const GraceSwitching: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-12)" }}>
      <Inspect card={<InspectCard kind={{ Glyph: Box, label: "Job" }} title="First card" />} name="First">
        <button type="button">First</button>
      </Inspect>
      <Inspect card={<InspectCard kind={{ Glyph: Box, label: "Job" }} title="Second card" />} name="Second">
        <button type="button">Second</button>
      </Inspect>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const first = canvas.getByRole("button", { name: "First" });
    const second = canvas.getByRole("button", { name: "Second" });
    const undo = timing("100ms", "300ms");
    try {
      await userEvent.hover(first);
      await waitFor(() => expect(screen.getByText("First card")).toBeVisible());
      // From here the delay is longer than the check below waits, so only the switch rule can explain the second card in time.
      document.documentElement.style.setProperty("--tooltip-delay", "3000ms");
      await userEvent.unhover(first);
      await userEvent.hover(second);
      await waitFor(() => expect(screen.getByText("Second card")).toBeVisible(), { timeout: 500 });
      await waitFor(() => expect(screen.queryByText("First card")).not.toBeInTheDocument());
    } finally {
      undo();
    }
  },
};

/** The pointer can cross the gap into the card and stay there; leaving the card then closes it. */
export const PointerCanEnterTheCard: Story = {
  render: () => (
    <Inspect
      card={
        <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Hold here">
          <button type="button">Inside</button>
        </InspectCard>
      }
      name="Job"
    >
      <button type="button">Trigger</button>
    </Inspect>
  ),
  play: async ({ canvas, userEvent }) => {
    const undo = timing("100ms");
    try {
      const trigger = canvas.getByRole("button", { name: "Trigger" });
      await userEvent.hover(trigger);
      await waitFor(() => expect(screen.getByRole("tooltip")).toBeVisible());
      const inside = screen.getByRole("button", { name: "Inside" });
      await userEvent.unhover(trigger);
      await userEvent.hover(inside);
      // Past the time the trigger's own leave would have closed it.
      await wait(400);
      await expect(screen.getByRole("tooltip")).toBeVisible();
      await userEvent.unhover(inside);
      await waitFor(() => expect(screen.queryByRole("tooltip")).not.toBeInTheDocument());
    } finally {
      undo();
    }
  },
};

/** Where `pin` is on, a press pins it: it becomes a named dialog, takes focus, and stays when the pointer leaves. */
export const PressPins: Story = {
  render: () => (
    <Inspect
      card={
        <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Pinned">
          <button type="button">Act</button>
        </InspectCard>
      }
      name="Job card"
    >
      <button type="button">Trigger</button>
    </Inspect>
  ),
  play: async ({ canvas, userEvent }) => {
    const trigger = canvas.getByRole("button", { name: "Trigger" });
    await userEvent.click(trigger);
    const dialog = await screen.findByRole("dialog", { name: "Job card" });
    await expect(dialog).toBeVisible();
    await expect(dialog.contains(document.activeElement)).toBe(true);
    await userEvent.unhover(trigger);
    await wait(400);
    await expect(screen.getByRole("dialog", { name: "Job card" })).toBeVisible();
  },
};

const pressed = fn();

/** Where `pin` is off the press stays the trigger's own act: it runs, and no dialog opens. */
export const PressDoesNotPinWhereItIsAnAct: Story = {
  render: () => (
    <Inspect card={card} name="Job" pin={false}>
      <button type="button" onClick={pressed}>
        Open
      </button>
    </Inspect>
  ),
  play: async ({ canvas, userEvent }) => {
    pressed.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Open" }));
    await wait(100);
    await expect(pressed).toHaveBeenCalledTimes(1);
    await expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  },
};

/** Esc closes a pinned card and returns focus to the trigger; a press outside closes it too. */
export const EscAndOutsidePressClose: Story = {
  render: () => (
    <div>
      <Inspect
        card={
          <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Closable">
            <button type="button">Act</button>
          </InspectCard>
        }
        name="Job card"
      >
        <button type="button">Trigger</button>
      </Inspect>
      <p>Elsewhere on the page</p>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const trigger = canvas.getByRole("button", { name: "Trigger" });
    await userEvent.click(trigger);
    await screen.findByRole("dialog", { name: "Job card" });
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await expect(trigger).toHaveFocus();

    // Pinned again, then a press somewhere else.
    await userEvent.click(trigger);
    await screen.findByRole("dialog", { name: "Job card" });
    await userEvent.click(canvas.getByText("Elsewhere on the page"));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  },
};

/** The keys that work on the item are a chip and a verb each, and the footer always ends in Esc. */
export const KeysFooter: StoryObj<typeof InspectCard> = {
  render: () => (
    <InspectCard
      kind={{ Glyph: Box, label: "Job" }}
      title="Keys"
      keys={[
        { key: "Enter", verb: "Open" },
        { act: "close", verb: "Dismiss" },
      ]}
    />
  ),
  play: async ({ canvas }) => {
    const items = canvas.getByRole("list", { name: "Keys" }).querySelectorAll("li");
    await expect(items[0]).toHaveTextContent("Open");
    await expect(items[1]).toHaveTextContent("Dismiss");
    await expect(items[items.length - 1]).toHaveTextContent(/Esc/);
  },
};

/** The card eases in only where motion is allowed; reduced motion is the stories' default, so it draws still. */
export const ReducedMotionDrawsStill: StoryObj<typeof InspectCard> = {
  render: () => <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Still" />,
  play: async ({ canvasElement }) => {
    const drawn = canvasElement.querySelector(".armada-inspect-card")!;
    await expect(getComputedStyle(drawn).animationName).toBe("none");
  },
};

/** With motion allowed the same card carries its entrance. */
export const MotionEasesIn: StoryObj<typeof InspectCard> = {
  parameters: { motion: "on" },
  render: () => <InspectCard kind={{ Glyph: Box, label: "Job" }} title="Eases in" />,
  play: async ({ canvasElement }) => {
    const drawn = canvasElement.querySelector(".armada-inspect-card")!;
    await expect(getComputedStyle(drawn).animationName).toBe("armada-inspect-in");
  },
};
