import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent } from "storybook/test";
import { Textarea } from "./Textarea";

const meta: Meta<typeof Textarea> = {
  title: "Primitives/Textarea",
  component: Textarea,
};
export default meta;

type Story = StoryObj<typeof Textarea>;

/** A card — the ground a field's sunken well is measured against. */
function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)",
      }}
    >
      {children}
    </div>
  );
}

const BRIEF =
  "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.";

/** At rest: `--bg-sunken` well, `--border-default` edge, `--fg-default` text. */
export const Default: Story = {
  args: { label: "Brief", defaultValue: BRIEF },
  render: (args) => (
    <Card>
      <Textarea {...args} />
    </Card>
  ),
};

/** Placeholder is `--fg-subtle`, the same step as a timestamp. */
export const Placeholder: Story = {
  args: { label: "Brief", placeholder: BRIEF },
  render: (args) => (
    <Card>
      <Textarea {...args} />
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, no glow. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Textarea label="Brief" defaultValue={BRIEF} data-preview-focus="" />
    </Card>
  ),
};

/** `--status-completed-failed` border, and the message below it in `--text-xs`. */
export const Invalid: Story = {
  render: () => (
    <Card>
      <Textarea label="Brief" invalid message="A job needs a brief. Write what the work is." />
    </Card>
  ),
};

/** `--fg-subtle` text with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Textarea label="Brief" defaultValue={BRIEF} disabled />
    </Card>
  ),
};

/**
 * Height is a row count. Three rows is the default and the well the M1 mockup
 * draws; a caller that needs more passes `rows`, which is the only thing that
 * changes the height — there is no drag handle.
 */
export const Rows: Story = {
  render: () => (
    <Card>
      <Textarea label="Brief" rows={6} defaultValue={BRIEF} />
    </Card>
  ),
};

/**
 * Past the row count, the well scrolls rather than growing. Nothing counts the
 * characters: no brief length limit is stated anywhere, and a counter would be
 * the first place one was invented.
 */
export const Overflowing: Story = {
  render: () => (
    <Card>
      <Textarea
        label="Brief"
        defaultValue={`${BRIEF} The retry ceiling is three, set where the transport is configured rather than at the call site, and moving it is a separate job. What is in scope is the coalescing: one refresh in flight, every waiter parked on it.`}
      />
    </Card>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists in `packages/tokens` — one `:root` block, nothing keyed to a theme —
 * so this renders dark. Written so the gap is visible rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Card>
        <Textarea label="Brief" defaultValue={BRIEF} />
      </Card>
    </div>
  ),
};

function Held({ about, controlled }: { about: string; controlled: boolean }) {
  const [value, setValue] = useState("");
  return controlled ? (
    <Textarea aria-label="Note" keep={about} value={value} onChange={(event) => setValue(event.target.value)} />
  ) : (
    <Textarea aria-label="Note" keep={about} />
  );
}

/** A panel that is left and come back to: the field is unmounted and mounted again, on the same key and on another. */
function Panels({ controlled }: { controlled: boolean }) {
  const [shown, setShown] = useState(true);
  const [about, setAbout] = useState("panel:a");
  return (
    <Card>
      <button onClick={() => setShown((was) => !was)}>Toggle</button>
      <button onClick={() => setAbout("panel:b")}>Other</button>
      {shown ? <Held about={about} controlled={controlled} /> : null}
    </Card>
  );
}

const keepsWhatWasTyped: Story["play"] = async ({ canvas }) => {
  await userEvent.type(canvas.getByRole("textbox", { name: "Note" }), "half a thought");
  await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
  await expect(canvas.queryByRole("textbox", { name: "Note" })).toBeNull();
  await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
  await expect(canvas.getByRole("textbox", { name: "Note" })).toHaveValue("half a thought");
  await userEvent.click(canvas.getByRole("button", { name: "Other" }));
  await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
  await userEvent.click(canvas.getByRole("button", { name: "Toggle" }));
  await expect(canvas.getByRole("textbox", { name: "Note" })).toHaveValue("");
};

/** Leave the panel and come back and the text is there; another key starts empty. */
export const KeepsWhatWasTyped: Story = { render: () => <Panels controlled={false} />, play: keepsWhatWasTyped };

/** The same, for a field whose host holds the value and lost it with the panel. */
export const KeepsWhatAHostLost: Story = { render: () => <Panels controlled />, play: keepsWhatWasTyped };
