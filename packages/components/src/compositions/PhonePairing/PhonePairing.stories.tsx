import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { Card, CardContent } from "../../primitives/Card/Card";
import { PhonePairing } from "./PhonePairing";

const URL = "https://studio.tail1a2b3c.ts.net/pair?code=9f2c41d07be3a56c18d4e0f7a3b25c69";

const meta: Meta<typeof PhonePairing> = {
  title: "Compositions/Phone pairing",
  component: PhonePairing,
  args: {
    phones: [
      { id: "p1", name: "Nick's iPhone", seen: "2 minutes ago" },
      { id: "p2", name: "iPad", seen: "3 days ago" },
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

export const Paired: Story = {
  play: async ({ canvasElement, args }) => {
    const body = within(canvasElement);
    await userEvent.click(body.getByRole("button", { name: "Unpair iPad" }));
    await expect(args.onUnpair).toHaveBeenCalledWith("p2");
  },
};

export const NothingPaired: Story = { args: { phones: [] } };

export const CodeShown: Story = { args: { url: URL, secondsLeft: 272 } };

export const Claimed: Story = {
  args: { url: URL, secondsLeft: 211, claim: { device: "Pixel 9" } },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Confirm" }));
    await expect(args.onConfirm).toHaveBeenCalledTimes(1);
  },
};

export const GatewayNotRunning: Story = { args: { problem: { kind: "not_running" } } };

export const TailscaleSignedOut: Story = {
  args: { problem: { kind: "tailscale", said: "Tailscale is logged out. Run tailscale up and sign in." } },
};
