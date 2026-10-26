import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Switch, SwitchControl } from "./Switch";

const meta: Meta<typeof Switch> = {
  title: "Primitives/Switch",
  component: Switch,
};
export default meta;

type Story = StoryObj<typeof Switch>;

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-2)",
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

/** On: the lit track says ON, with the knob at the far end. */
export const On: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked>Escalate on stall</Switch>
    </Card>
  ),
  play: async ({ canvas }) => {
    const on = canvas.getByRole("switch", { name: "Escalate on stall" });
    await expect(on).toBeChecked();
    await expect(on.closest("label")).not.toHaveAttribute("data-size");
    await expect(canvas.getByText("ON")).toBeInTheDocument();
    await expect(canvas.getByText("OFF")).toBeInTheDocument();
  },
};

/** Off: the dark track says OFF, with the knob at the near end. */
export const Off: Story = {
  render: () => (
    <Card>
      <Switch>Auto-approve small diffs</Switch>
    </Card>
  ),
};

/**
 * The track says what the setting is in its own words, and widens to the longer of the two. The switch keeps its name and its
 * checked state, and a press flips it.
 */
export const Words: Story = {
  render: () => (
    <Card>
      <Switch words={{ on: "24H", off: "12H" }} defaultChecked>
        24-hour clock
      </Switch>
    </Card>
  ),
  play: async ({ canvas, userEvent }) => {
    const clock = canvas.getByRole("switch", { name: "24-hour clock" });
    await expect(clock).toBeChecked();
    await expect(canvas.getByText("24H")).toBeInTheDocument();
    await expect(canvas.getByText("12H")).toBeInTheDocument();
    await expect(canvas.queryByText("ON")).toBeNull();
    await userEvent.click(clock);
    await expect(clock).not.toBeChecked();
  },
};

/** A band's or a bar's switch, one badge tall. A row's has no size. */
export const Small: Story = {
  render: () => (
    <Card>
      <Switch size="sm" defaultChecked>
        Autopilot
      </Switch>
    </Card>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("switch", { name: "Autopilot" }).closest("label")).toHaveAttribute("data-size", "sm");
  },
};

/** The bare control, for a row whose text does something else. Only the control toggles, and `label` names it. */
export const Bare: Story = {
  args: {},
  render: () => {
    const onChange = fn();
    return (
      <Card>
        <button type="button" onClick={onChange}>
          Open Dusk
        </button>
        <SwitchControl label="Dusk" size="sm" words={{ on: "Y", off: "N" }} defaultChecked onChange={(event) => onChange(event.target.checked)} />
      </Card>
    );
  },
  play: async ({ canvas, userEvent }) => {
    const control = canvas.getByRole("switch", { name: "Dusk" });
    await expect(control).toBeChecked();
    await expect(control.closest("label")).toHaveAttribute("data-size", "sm");
    await expect(control.closest("label")).toHaveAttribute("data-control", "");
    await expect(canvas.getByText("Y")).toBeInTheDocument();
    await userEvent.click(control);
    await expect(control).not.toBeChecked();
  },
};

/** A 2px `--accent` ring at 2px offset, on the track. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked data-preview-focus="">
        Escalate on stall
      </Switch>
    </Card>
  ),
};

/** `--fg-subtle` with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked disabled>
        Escalate on stall
      </Switch>
      <Switch disabled>Auto-approve small diffs</Switch>
    </Card>
  ),
  /**
   * **Disabled is set back, never opacity** — which means a disabled switch
   * looks like a quiet one, and nothing about how it is drawn says it is
   * inert. Pressing it is the only thing that does.
   *
   * Pressed by its label, because that is where a person presses a switch: the
   * whole row is the `label`, so a click anywhere in it forwards to the input.
   * A switch that took that press while disabled would flip a setting the
   * surface had already said was not available.
   */
  play: async ({ canvas, userEvent }) => {
    const on = canvas.getByRole("switch", { name: "Escalate on stall" });
    await expect(on).toBeChecked();

    await userEvent.click(canvas.getByText("Escalate on stall"));
    await expect(on).toBeChecked();
  },
};

/** The description says what happens on and what happens off. */
export const WithADescription: Story = {
  render: () => (
    <Card>
      <Switch
        defaultChecked
        description="A drone that stops reporting for 12 minutes reaches your phone. Off, it waits on the Alerts queue."
      >
        Escalate on stall
      </Switch>
      <Switch description="Armada reads the config repo each launch. Local edits made since the last push are kept.">
        Pull the Kit on startup
      </Switch>
    </Card>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists in `packages/tokens`, so this renders dark — written so the gap is
 * visible rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Card>
        <Switch defaultChecked>Escalate on stall</Switch>
      </Card>
    </div>
  ),
};
