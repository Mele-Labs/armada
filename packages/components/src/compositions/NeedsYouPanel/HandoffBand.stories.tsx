import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { HandoffBand } from "./HandoffBand";

/** A hand off that did not start: the failed band on the composer, with the reason Fleet recorded and Retry and Dispatch job. */
const meta: Meta<typeof HandoffBand> = {
  title: "Compositions/Handoff band",
  component: HandoffBand,
  args: { onDismiss: fn(), onRetry: fn(), onDispatch: fn() },
  decorators: [
    (Story) => (
      <div className="armada-session-frame__centre" style={{ position: "relative", marginBlockStart: "calc(var(--space-12) * 3)", width: "calc(var(--space-12) * 10)" }}>
        <Story />
        <textarea aria-label="Message" />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof HandoffBand>;

export const WithReason: Story = { args: { reason: "old job did not end: the close was refused" } };

/** Where Fleet recorded no reason the band says only that it failed. */
export const WithoutReason: Story = {};
