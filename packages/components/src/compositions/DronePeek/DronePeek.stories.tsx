import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import type { DroneTurn } from "../DroneTurns/DroneTurns";
import { DronePeek } from "./DronePeek";

/**
 * One Drone, small: its name, state and run time, the tail of its transcript,
 * and the message box. Open goes to the whole Drone.
 *
 * The words, the rows and the press are the caller's. This decides the card,
 * the short scroller pinned to the tail, and the order of the three.
 */
const meta: Meta<typeof DronePeek> = {
  title: "Compositions/Drone peek",
  component: DronePeek,
  decorators: [
    (Story) => (
      <div style={{ width: "var(--w-dock)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof DronePeek>;

const TURNS: DroneTurn[] = [
  { id: "0", at: "10:15:01", who: "armada", kind: "instructed", said: "Open a Drone's Job from its row." },
  { id: "1", at: "10:15:04", who: "drone", kind: "said", said: "Reading the files it touches first." },
  { id: "2", at: "10:15:06", who: "drone", kind: "called", subject: "Read", detail: "packages/screens/src/running-rows.tsx" },
  { id: "3", at: "10:15:20", who: "drone", kind: "unrecognised", quiet: true, thought: { of: "thinking", tokens: 620 } },
  { id: "4", at: "10:15:31", who: "drone", kind: "said", said: "The change belongs in running-rows.tsx. Writing it now." },
  { id: "5", at: "10:16:02", who: "drone", kind: "called", subject: "Edit", detail: "packages/screens/src/running-rows.tsx" },
  { id: "6", at: "10:16:40", who: "drone", kind: "called", subject: "Bash", detail: "pnpm -C packages/screens exec vitest run" },
  { id: "7", at: "10:17:12", who: "drone", kind: "unrecognised", quiet: true, thought: { of: "thinking", tokens: 1030 } },
];

const MESSAGE = { value: "", onChange: fn(), onSend: fn() };

/** Writing: the tail is followed, and the last thinking run says `Working`. */
export const Running: Story = {
  args: {
    title: "Drone on T6",
    state: "running",
    stateSays: "Running",
    ranFor: "2m 11s",
    turns: TURNS,
    live: true,
    onOpen: fn(),
    message: MESSAGE,
  },
};

/** Stopped on its own: it opens on its last turns, not its first. */
export const Failed: Story = {
  args: {
    ...Running.args,
    state: "failed",
    stateSays: "Failed",
    ranFor: "16m 00s",
    live: false,
    turns: [...TURNS, { id: "8", at: "10:31:00", who: "drone", kind: "ended", subject: "15 turns · $0.72" }],
  },
};

/**
 * Nothing written yet, and no box: the caller had no Drone to reach. **The
 * transcript draws nothing** — an empty slot stays empty, never a sentence.
 */
export const NothingWritten: Story = {
  args: {
    title: "Drone on T7",
    state: "running",
    stateSays: "Running",
    turns: [],
    live: true,
    onOpen: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Drone on T7")).toBeVisible();
    await expect(canvas.queryByRole("note")).toBeNull();
    await expect(canvas.queryByText(/nothing/i)).toBeNull();
  },
};
