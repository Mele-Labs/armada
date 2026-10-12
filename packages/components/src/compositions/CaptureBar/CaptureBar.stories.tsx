import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { ACTION } from "../../generated/actions";
import { CaptureBar } from "./CaptureBar";

/** The binding is the registry's own, as Studio capture's is. */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

const meta: Meta<typeof CaptureBar> = {
  title: "Compositions/Capture bar",
  component: CaptureBar,
  args: {
    run: "web_dev",
    address: "http://127.0.0.1:41207",
    studio: "The checkout flow",
    serving: true,
    armed: false,
    framesRefused: 0,
    onArm: fn(),
    onReload: fn(),
    onFollowRefused: fn(),
    binding: BINDING,
  },
};
export default meta;

type Story = StoryObj<typeof CaptureBar>;

/** Serving: the Run, the address in full, and the Studio a Note will land on. */
export const Serving: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.getByText("http://127.0.0.1:41207")).toBeVisible();
    await expect(canvas.getByText("Notes land on The checkout flow")).toBeVisible();
    // No address field, no Back and no Forward: this is not a browser.
    await expect(canvas.queryByRole("textbox")).toBeNull();
  },
};

/**
 * A Job's server, opened from its run sheet to be walked: no Studio, so the bar
 * says what it is and offers no Capture — a Note would have nowhere to land.
 */
export const Walking: Story = {
  args: { run: "mock", address: "http://localhost:41311", studio: null },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Walking a Job's server")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Capture" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Reload" })).toBeEnabled();
  },
};

/**
 * A Job's server, walked: notes go to the Job, and wait there until it is sent
 * back. ⌥⌘A arms it as well as the registry's binding.
 */
export const WalkingAJob: Story = {
  args: { run: "mock", address: "http://localhost:41311", studio: null, job: "44-try-a-stacked-run-beside-the-canvas" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Notes go to 44-try-a-stacked-run-beside-the-canvas")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Capture" })).toBeEnabled();
  },
};

/** Armed: a press on the page points rather than acts. */
export const Capturing: Story = {
  args: { armed: true },
};

/**
 * A refused address is drawn in full and never followed. One act offers it to
 * the browser this person already uses.
 */
export const Refused: Story = {
  args: {
    refused: {
      address: "https://accounts.google.com/o/oauth2/v2/auth?client_id=1",
      said: "Navigation",
      offerable: true,
    },
  },
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open in browser" }));
    await expect(args.onFollowRefused).toHaveBeenCalled();
  },
};

/** A subframe off the origin is counted rather than named: a page draws many. */
export const FramesRefused: Story = {
  args: { framesRefused: 3 },
};

/**
 * The run ended. **Capture is closed and the window loads nothing further** —
 * a loopback port is not an identity, and anything may bind it once the server
 * exits.
 */
export const RunEnded: Story = {
  args: { serving: false },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Capture" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Reload" })).toBeDisabled();
  },
};

/**
 * A Session's question, folded so the page can be used. The second row says it waits, and one press
 * draws it over the page again.
 */
export const QuestionFolded: Story = {
  args: {
    run: "Findings page",
    address: "http://localhost:5173",
    studio: null,
    session: "Findings page",
    question: "Should the count sit beside the title or under it?",
    onShowQuestion: fn(),
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.getByText(/Question waiting: Should the count sit/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Show question" }));
    await expect(args.onShowQuestion).toHaveBeenCalled();
  },
};

/**
 * A Session's page, its thread drawn beside it. The bar names the Session as it would a Job, and one
 * press folds the panel away so the page has the window — `capture-window.md`, *The panel beside the page*.
 */
export const WalkingASession: Story = {
  args: {
    run: "Findings page",
    address: "http://localhost:5173",
    studio: null,
    session: "Findings page",
    onApprove: fn(),
    panel: { kind: "session", open: true, onToggle: fn() },
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.getByText("Notes go to Findings page")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Hide Session" }));
    await expect(args.panel?.onToggle).toHaveBeenCalledWith(false);
  },
};

/** A Job's server, its panel folded away. The press names the kind, so it says what comes back. */
export const JobPanelFolded: Story = {
  args: {
    run: "mock",
    address: "http://localhost:41311",
    studio: null,
    job: "44-try-a-stacked-run-beside-the-canvas",
    panel: { kind: "job", open: false, onToggle: fn() },
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Hide Job" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Show Job" }));
    await expect(args.panel?.onToggle).toHaveBeenCalledWith(true);
  },
};

/** A Studio's window has no panel: its Studio is read on Bridge's window. */
export const StudioHasNoPanel: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /Session|Job/ })).toBeNull();
  },
};
