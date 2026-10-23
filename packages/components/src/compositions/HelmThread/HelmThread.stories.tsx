import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { HelmThread, type HelmThreadRow } from "./HelmThread";

/** Helm's own conversation, drawn at the dock's own width. */
const meta: Meta<typeof HelmThread> = {
  title: "Compositions/Helm thread",
  component: HelmThread,
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <HelmThread {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmThread>;

const asked: HelmThreadRow = { id: "1", at: "14:29:40", actor: "you", message: "Why did the checkout Session stall?" };
const replied: HelmThreadRow = {
  id: "2",
  at: "14:29:52",
  actor: "helm",
  message: "The Session is waiting on a command it was not given: cargo nextest run -p store.",
};
const cost: HelmThreadRow = { ...replied, id: "3", message: "It needs an answer in the dock above.", meta: "~$0.01 · 4 turns" };

/** A conversation with a reply already in, nothing in flight. */
export const AtRest: Story = { args: { rows: [asked, cost] } };

/** Nothing asked yet, or the thread just cleared: **nothing is drawn**, no sentence and no note. */
export const Empty: Story = {
  args: { rows: [] },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.queryByRole("note")).toBeNull();
    await expect(canvasElement.textContent).toBe("");
  },
};

/** A message just went out and Helm's own rows are still arriving. */
export const MidReply: Story = {
  args: { rows: [asked], replying: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Helm is replying…");
  },
};

/** Fleet is unreachable. What already arrived stays on screen. */
export const FleetUnreachable: Story = {
  args: { rows: [asked, cost], notice: "Fleet is not connected. What Helm already said is still here." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Fleet is not connected");
  },
};

/**
 * Helm's own reply carries the same subset `Prose` draws everywhere else —
 * a heading, a list, inline code and bold, not the literal asterisks a
 * naive paragraph split left on screen. `#1167`.
 */
export const AMarkdownReply: Story = {
  args: {
    rows: [
      { id: "1", at: "14:29:40", actor: "you", message: "Why did the checkout Session stall?" },
      {
        id: "2",
        at: "14:29:52",
        actor: "helm",
        message:
          "# What stalled it\n\n" +
          "The Session is waiting on a command it was **never given**:\n\n" +
          "- `cargo nextest run -p store`\n" +
          "- a workflow step that names it\n\n" +
          "Add the step, or give the Session the command directly.",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText(/\*\*never given\*\*/)).not.toBeInTheDocument();
    await expect(canvas.getByText("What stalled it")).toBeInTheDocument();
    await expect(canvas.getByText("cargo nextest run -p store").tagName).toBe("CODE");
    await expect(canvas.getByText("never given").tagName).toBe("STRONG");
  },
};

/** A one-pixel picture, so a thumbnail has something to draw. */
const PIXEL = "data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs=";

/** What a message carried is drawn under its words: a picture as a thumbnail named for the file, and a file with none as its name. */
export const SentWithFiles: Story = {
  args: {
    rows: [
      {
        ...asked,
        message: "Do these explain the stale clock?",
        files: [
          { id: "f1", name: "Screenshot.png", src: PIXEL },
          { id: "f2", name: "clock.log" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Screenshot\.png/ })).toBeInTheDocument();
    await expect(canvas.getByText("clock.log")).toBeInTheDocument();
  },
};

/** A sketch sent from this window is drawn as the sketch, by its title. */
export const SentWithASketch: Story = {
  args: { rows: [{ ...asked, message: "Like this?", sketches: [{ id: "k1", title: "pin the clock" }] }] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("pin the clock")).toBeInTheDocument();
  },
};

/** Issues and pull requests the message tagged stand in its line, where `@` or `#` was typed. */
export const SentWithTags: Story = {
  args: {
    rows: [
      {
        ...asked,
        message: "see @#2214 Sessions list drops a quiet Session and @#2207 Stop the Drone count drifting",
        tags: [
          { kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" },
          { kind: "pull_request", id: "2207", title: "#2207 Stop the Drone count drifting", url: "https://forge.invalid/pull/2207" },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("#2214 Sessions list drops a quiet Session")).toBeInTheDocument();
    await expect(canvas.getByText("#2207 Stop the Drone count drifting")).toBeInTheDocument();
  },
};

/** A message that was only a picture has no words, and its row is the picture. */
export const SentWithNoWords: Story = {
  args: { rows: [{ ...asked, message: "", files: [{ id: "f1", name: "Screenshot.png", src: PIXEL }] }] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Screenshot\.png/ })).toBeInTheDocument();
  },
};
