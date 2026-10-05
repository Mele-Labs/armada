import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, within } from "storybook/test";

import { JobTimeline } from "./JobTimeline";
import type { JobTimelineBar } from "./JobTimeline";

const MIN = 60_000;
const NOW = Date.parse("2026-10-05T18:00:00Z");
const at = (min: number) => NOW - (180 - min) * MIN;

const bar = (id: string, title: string, status: string, from: number, to: number): JobTimelineBar => ({
  id,
  card: { job: id, title },
  status,
  from,
  to,
});
const bars: JobTimelineBar[] = [
  bar("a", "Migrate the job store", "running", at(0), NOW),
  bar("b", "Cache the manifest read", "completed_success", at(30), at(80)),
  bar("c", "Document the cache keys", "completed_success", at(85), at(120)),
  bar("d", "Benchmark the cached read", "killed", at(60), at(100)),
];

const meta: Meta<typeof JobTimeline> = {
  title: "Compositions/Job timeline",
  component: JobTimeline,
  parameters: { layout: "padded" },
  render: (args) => {
    const [playhead, setPlayhead] = useState(args.playhead);
    return <JobTimeline {...args} playhead={playhead} onPlayhead={setPlayhead} />;
  },
  args: {
    bars,
    now: NOW,
    playhead: at(90),
    dispatches: [
      { parent: "a", child: "b", at: at(28) },
      { parent: "b", child: "c", at: at(70) },
      { parent: "b", child: "d", at: at(55) },
    ],
  },
};
export default meta;

type Story = StoryObj<typeof JobTimeline>;

export const Scrubbed: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("slider", { name: "Playhead" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "More time" })).toBeInTheDocument();
  },
};
