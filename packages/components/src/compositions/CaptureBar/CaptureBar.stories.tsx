import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor } from "storybook/test";

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
 * A server opened to be walked: no Studio, so the bar
 * says what it is and offers no Capture — a Note would have nowhere to land.
 */
export const Walking: Story = {
  args: { run: "mock", address: "http://localhost:41311", studio: null },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Walking a server")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Capture" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Reload" })).toBeEnabled();
  },
};

/** The Session's title opens the Session in Bridge where the window offers it. */
export const WalkingASessionLinked: Story = {
  args: { run: "mock", address: "http://localhost:41311", studio: null, session: "Findings page", onOpenOwner: fn() },
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Findings page" }));
    await expect(args.onOpenOwner).toHaveBeenCalledOnce();
  },
};

/**
 * A page a Session showed with no title of its own: the address is drawn once, in full, and no name
 * repeats it. The Session is named by its own name and opens it.
 */
export const ShownBySession: Story = {
  args: {
    run: undefined,
    address: "http://localhost:47342/?walk=cockpit-pending",
    studio: null,
    session: "Capture Window Titlebar",
    onOpenOwner: fn(),
    onApprove: fn(),
  },
  play: async ({ args, canvas }) => {
    await expect(canvas.getAllByText("http://localhost:47342/?walk=cockpit-pending")).toHaveLength(1);
    await userEvent.click(canvas.getByRole("button", { name: "Capture Window Titlebar" }));
    await expect(args.onOpenOwner).toHaveBeenCalledOnce();
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
 * A Session's page, its thread drawn beside it. The bar names the Session, and one
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
    await userEvent.click(canvas.getByRole("button", { name: "Hide Job" }));
    await expect(args.panel?.onToggle).toHaveBeenCalledWith(false);
  },
};

/** A Studio's window has no panel: its Studio is read on Bridge's window. */
export const StudioHasNoPanel: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /Session|Job/ })).toBeNull();
  },
};

/** A walk Bridge plays on a page, at step 2 of 5 and ringed, so Next has something to act on. */
const PLAYING = {
  name: "Portal",
  at: 2,
  of: 5,
  said: "Sign in",
  autoplay: false,
  ready: true,
  done: false,
  onBack: fn(),
  onNext: fn(),
  onAutoplay: fn(),
  onReplay: fn(),
};

/**
 * Autoplay stopped before a step that acts on a page off this machine: the Next key pulses, and its
 * tooltip says why. Autoplay is off, and Next runs the step.
 */
export const WaitingOnNext: Story = {
  args: {
    run: "Staging",
    address: "https://staging.example.com",
    studio: null,
    session: "Staging",
    walk: { ...PLAYING, guide: { shown: true, onShow: fn(), waiting: true } },
  },
  play: async ({ args, canvas }) => {
    const next = canvas.getByRole("button", { name: /Next/ });
    await expect(next).toHaveAttribute("data-waiting");
    await userEvent.hover(next);
    await waitFor(() => expect(canvas.getByText("Autoplay stops before a step that acts on this page")).toBeVisible());
    await userEvent.click(next);
    await expect(args.walk?.onNext).toHaveBeenCalled();
  },
};

/** A step with nothing to wait for: the Next key is plain. */
export const NotWaiting: Story = {
  args: { ...WaitingOnNext.args, walk: { ...PLAYING, guide: { shown: true, onShow: fn() } } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Next/ })).not.toHaveAttribute("data-waiting");
  },
};

/**
 * The page is on a sign-in provider: a glyph with a tooltip, no not-found glyph, and Next is not
 * ready. Signing in wins over the waiting key, so Next does not pulse.
 */
export const SigningIn: Story = {
  args: {
    run: "Staging",
    address: "https://staging.example.com",
    studio: null,
    session: "Staging",
    walk: { ...PLAYING, ready: false, guide: { shown: true, onShow: fn(), signingIn: true, waiting: true } },
  },
  play: async ({ canvas }) => {
    const glyph = canvas.getByRole("img", { name: "Signing in" });
    await expect(glyph).toBeVisible();
    await userEvent.hover(glyph);
    await waitFor(() => expect(canvas.getByText("Signing in")).toBeVisible());
    await expect(canvas.queryByRole("img", { name: "Not found" })).toBeNull();
    await expect(canvas.getByRole("button", { name: /Next/ })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: /Next/ })).not.toHaveAttribute("data-waiting");
  },
};

/** A step whose target is not found draws the not-found glyph, and signing in does not. */
export const TargetNotFound: Story = {
  args: { ...SigningIn.args, walk: { ...PLAYING, ready: false, guide: { shown: true, onShow: fn(), stopped: "a button named “Report”" } } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Not found" })).toBeVisible();
    await expect(canvas.queryByRole("img", { name: "Signing in" })).toBeNull();
  },
};
