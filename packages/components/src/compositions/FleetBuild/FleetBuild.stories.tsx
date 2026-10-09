import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { FleetBuildSection } from "./FleetBuild";
import type { FleetBuild } from "./FleetBuild";

/**
 * The Fleet panel's build section: the build Fleet runs on, the one act, which
 * follows the selection, and where the build stands against main.
 */
const meta: Meta<typeof FleetBuildSection> = {
  title: "Compositions/Fleet build",
  component: FleetBuildSection,
  decorators: [
    (Story) => (
      <div style={{ width: 200 }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof FleetBuildSection>;

const base = (over: Partial<FleetBuild>): { build: FleetBuild } => ({
  build: { on: "main", working: null, drones: [], onChoose: fn(), onRestart: fn(), ...over },
});

/** On Main and behind it, the button moves Fleet to the latest main. */
export const MainBehind: Story = {
  args: base({ position: { ahead: 0, behind: 3 } }),
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("img", { name: "3 commits behind main" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Update to main" }));
    await expect(args.build.onRestart).toHaveBeenCalledWith(false);
  },
};

/** On Main and level with it, the button is there and cannot be pressed. */
export const MainAligned: Story = {
  args: base({ position: { ahead: 0, behind: 0 } }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Aligned with main" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Update to main" })).toBeDisabled();
  },
};

/** A position that cannot be counted draws no mark, and does not claim to be level. */
export const MainUncounted: Story = {
  args: base({}),
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: "Aligned with main" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Update to main" })).toBeEnabled();
  },
};

/** On the preview, the button merges the in-flight branches again. */
export const PreviewAhead: Story = {
  args: base({ on: "preview", position: { ahead: 4, behind: 0 } }),
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("img", { name: "4 commits ahead of main" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Refresh preview" }));
    await expect(args.build.onRestart).toHaveBeenCalledWith(false);
  },
};

/** Choosing the other build restarts onto it. */
export const ChoosingTheOtherBuild: Story = {
  args: base({ position: { ahead: 0, behind: 0 } }),
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Build" }), "preview");
    await expect(args.build.onChoose).toHaveBeenCalledWith("preview", false);
  },
};

/** A restart under way: the ring turns where the figure was, and neither control can be pressed. */
export const Restarting: Story = {
  args: base({ working: "main", position: { ahead: 0, behind: 3 } }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Restarting Fleet" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Updating to main" })).toHaveAttribute("aria-disabled", "true");
    await expect(canvas.getByRole("combobox", { name: "Build" })).toBeDisabled();
  },
};

/** A restart that has named its stage: the words take the ring's place beside it, and the bare ring is gone. */
export const RestartingStage: Story = {
  args: base({ working: "preview", on: "preview", stage: "building_bridge", position: { ahead: 4, behind: 0 } }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Building Bridge");
    await expect(canvas.queryByRole("img", { name: "Restarting Fleet" })).toBeNull();
    await expect(canvas.queryByRole("img", { name: "4 commits ahead of main" })).toBeNull();
  },
};

/** The longest stage at the panel's narrowest, 160px: it neither runs past the section nor under the ring. */
export const LongestStageAtTheNarrowestPanel: Story = {
  args: base({ working: "main", stage: "reopening_bridge", position: { ahead: 0, behind: 3 } }),
  decorators: [
    (Story) => (
      <div style={{ width: 160 }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas, canvasElement }) => {
    const stage = canvas.getByRole("status");
    await expect(stage).toHaveTextContent("Reopening Bridge");
    const section = canvasElement.querySelector(".armada-fleet-build");
    const ring = stage.querySelector("svg");
    if (section === null || ring === null) throw new Error("the section and its ring are drawn");
    const words = document.createRange();
    words.selectNodeContents(stage);
    const sectionBox = section.getBoundingClientRect();
    const ringBox = ring.getBoundingClientRect();
    const lines = [...words.getClientRects()].filter((box) => box.left >= ringBox.right - 0.5);
    await expect(lines.length).toBe(1);
    const text = lines[0]!;
    await expect(text.left).toBeGreaterThanOrEqual(ringBox.right);
    await expect(text.right).toBeLessThanOrEqual(sectionBox.right);
    await expect(section.scrollWidth).toBeLessThanOrEqual(section.clientWidth);
  },
};

/** Switching from Main to the preview names what it is restarting onto. */
export const SwitchingToThePreview: Story = {
  args: base({ working: "preview", position: { ahead: 0, behind: 0 } }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Refreshing preview" })).toHaveAttribute("aria-disabled", "true");
  },
};

/** A Drone working asks first, lists the Jobs, and passes the person's yes as adopt. */
export const DronesWorking: Story = {
  args: base({
    position: { ahead: 0, behind: 3 },
    drones: [
      { id: "a", label: "Reword the empty states" },
      { id: "b", label: "Count drones from the roster" },
    ],
  }),
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Update to main" }));
    await expect(args.build.onRestart).not.toHaveBeenCalled();
    await expect(canvas.getByRole("dialog", { name: "Drones are working" })).toBeVisible();
    await expect(canvas.getByText("Reword the empty states")).toBeVisible();
    await expect(canvas.getByText("Count drones from the roster")).toBeVisible();
    await expect(canvas.getByText("Cannot be redirected until it finishes")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Restart" }));
    await expect(args.build.onRestart).toHaveBeenCalledWith(true);
  },
};

/** Cancelling the confirm sends nothing. */
export const DronesWorkingCancelled: Story = {
  args: base({ on: "preview", drones: [{ id: "a", label: "Reword the empty states" }] }),
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Build" }), "main");
    await userEvent.click(canvas.getByRole("button", { name: "Cancel" }));
    await expect(args.build.onChoose).not.toHaveBeenCalled();
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};

/** Why the last restart did not take, in the script's own words. */
export const Failed: Story = {
  args: base({
    position: { ahead: 0, behind: 3 },
    failed: "main cannot fast-forward to origin/main — it has diverged or a local change is in the way",
  }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("main cannot fast-forward to origin/main");
    await expect(canvas.getByRole("button", { name: "Update to main" })).toBeEnabled();
  },
};
