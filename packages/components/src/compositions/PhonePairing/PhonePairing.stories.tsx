import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { Card, CardContent } from "../../primitives/Card/Card";
import { PhonePairing } from "./PhonePairing";

const URL = "https://studio.tail1a2b3c.ts.net/pair?code=9f2c41d07be3a56c18d4e0f7a3b25c69";

const meta: Meta<typeof PhonePairing> = {
  title: "Compositions/Phone pairing",
  component: PhonePairing,
  args: {
    access: { on: true },
    onTurn: fn(),
    phones: [
      { id: "p1", name: "iPhone", seenAt: Date.now() - 2 * 60_000 },
      { id: "p2", name: "iPad", seenAt: Date.now() - 3 * 86_400_000 },
    ],
    onPair: fn(),
    onConfirm: fn(),
    onUnpair: fn(),
  },
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
        <Card>
          <CardContent>
            <Story />
          </CardContent>
        </Card>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PhonePairing>;

export const AccessOff: Story = {
  args: { access: { on: false } },
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    // Off is the switch and its line: no pairing and no phones, though some are paired.
    await expect(body.queryByRole("button", { name: "Pair a phone" })).toBeNull();
    await expect(body.queryByText("iPhone")).toBeNull();
    await userEvent.click(body.getByText("Phone access"));
    await expect(args.onTurn).toHaveBeenCalledWith(true);
  },
};

export const AccessTurning: Story = { args: { access: { on: false, turning: true } } };

export const Paired: Story = {
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    await userEvent.click(body.getByRole("button", { name: "Unpair iPad" }));
    await expect(args.onUnpair).toHaveBeenCalledWith("p2");
  },
};

export const NothingPaired: Story = { args: { phones: [] } };

export const CodeShown: Story = {
  args: { url: URL, secondsLeft: 272, onCopied: fn() },
  play: async ({ canvasElement, args }) => {
    const code = within(canvasElement).getByRole("img", { name: "Pairing code" });
    // A code is dark modules on a light ground: the drawing holds a path, not an empty square.
    await expect(code.querySelector("path")?.getAttribute("d")?.length).toBeGreaterThan(100);
    await expect(within(canvasElement).getByText("4:32")).toBeVisible();
    // Headless Chromium grants no clipboard permission, so the write is caught rather than read back.
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => void written.push(text) } });
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Copy link" }));
    await expect(written).toEqual([URL]);
    await expect(args.onCopied).toHaveBeenCalledWith("The link");
  },
};

export const Claimed: Story = {
  args: { url: URL, secondsLeft: 211, claim: { device: "Pixel 9" } },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Confirm" }));
    await expect(args.onConfirm).toHaveBeenCalledTimes(1);
  },
};

export const PortTaken: Story = {
  args: { problem: { kind: "said", said: "Port 8443 is in use by another program." } },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement);
    await expect(body.getByRole("switch", { name: /^Phone access/ })).toBeChecked();
    await expect(body.getByText("Port 8443 is in use by another program.")).toBeVisible();
    await expect(body.queryByRole("button", { name: "Pair a phone" })).toBeNull();
  },
};

export const GatewayNotRunning: Story = { args: { problem: { kind: "not_running" } } };

export const TailscaleSignedOut: Story = {
  args: { problem: { kind: "said", said: "Tailscale is not signed in on this Mac. Sign in to it, then start pairing again." } },
};

/** Nothing set up: every step not done, only the first with its press, and nothing to pair yet. */
export const WayInFromNothing: Story = {
  args: { phones: [], wayIn: { tailscale: "missing", served: "none", named: false }, onOpen: fn(), onFunnel: fn() },
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    await expect(body.getAllByRole("img", { name: "Not done yet" })).toHaveLength(4);
    await expect(body.queryByRole("button", { name: "Turn on Funnel" })).toBeNull();
    await expect(body.queryByRole("button", { name: "Pair a phone" })).toBeNull();
    await userEvent.click(body.getByRole("button", { name: "Get Tailscale" }));
    await expect(args.onOpen).toHaveBeenCalledWith("https://tailscale.com/download/mac");
  },
};

/** Signed in, Funnel off: Turn on Funnel is the press. */
export const WayInTurnOnFunnel: Story = {
  args: { phones: [], wayIn: { tailscale: "running", served: "none", named: false, address: "https://studio.tail1a2b3c.ts.net" }, onFunnel: fn() },
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    await expect(body.getAllByRole("img", { name: "Done" })).toHaveLength(2);
    await userEvent.click(body.getByRole("button", { name: "Turn on Funnel" }));
    await expect(args.onFunnel).toHaveBeenCalled();
  },
};

/** The tailnet has not allowed Funnel: Allow Funnel opens its page. */
export const WayInAllowFunnel: Story = {
  args: {
    phones: [],
    wayIn: { tailscale: "running", served: "none", named: false, funnel: { state: "approve", page: "https://login.tailscale.com/f/funnel?node=n1" } },
    onOpen: fn(),
  },
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    await userEvent.click(body.getByRole("button", { name: "Allow Funnel" }));
    await expect(args.onOpen).toHaveBeenCalledWith("https://login.tailscale.com/f/funnel?node=n1");
  },
};

/** `tailscale serve` already: phones can pair, and Funnel is still offered so they need no Tailscale. */
export const WayInOverTheTailnet: Story = {
  args: { wayIn: { tailscale: "running", served: "tailnet", named: false, address: "https://studio.tail1a2b3c.ts.net" } },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement);
    await expect(body.getByText(/^Phones reach this Mac now only with Tailscale on them/)).toBeTruthy();
    await expect(body.getByRole("button", { name: "Pair a phone" })).toBeTruthy();
  },
};

/** All done: one line. */
export const WayInDone: Story = {
  args: { wayIn: { tailscale: "running", served: "funnel", named: false, address: "https://studio.tail1a2b3c.ts.net" } },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement);
    await expect(body.queryByRole("region", { name: "Way in" })).toBeNull();
    await expect(body.getByText(/through Tailscale Funnel\.$/)).toBeTruthy();
  },
};

/** A tunnel of the owner's own, named in `pocket-address`: one line, no Tailscale steps. */
export const WayInNamed: Story = {
  args: { wayIn: { tailscale: "missing", served: "none", named: true, address: "https://armada.example.com" } },
};
