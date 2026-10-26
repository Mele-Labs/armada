import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import type { RunningApp } from "../RunControl/runs";
import { RunningPanel } from "./RunningPanel";

/**
 * A run that is starting or running: the checkout it is on, a row for each process with its state, its leased port and the way to open it, and a log
 * for each process under tabs.
 */
const meta: Meta<typeof RunningPanel> = {
  title: "Compositions/Running panel",
  component: RunningPanel,
};
export default meta;

type Story = StoryObj<typeof RunningPanel>;

const BROWSER: RunningApp = {
  id: "armada:main:mock",
  run: "Mock Bridge",
  kind: "browser",
  checkout: { kind: "main", branch: "main" },
  processes: [
    { id: "backend", name: "backend", port: 8001, phase: "ready", url: "http://localhost:8001", log: ["INFO started server", "INFO listening on 8001"] },
    { id: "frontend", name: "frontend", port: 5173, phase: "starting", url: "http://localhost:5173", log: ["VITE v6 ready"] },
    { id: "worker", name: "worker", phase: "ready", log: ["worker idle"] },
    { id: "legacy", name: "legacy", port: 9000, phase: "exited", url: "http://localhost:9000", log: ["stopped"] },
  ],
};

const APP: RunningApp = {
  id: "armada:job:bridge",
  run: "Bridge dev, scratch Fleet",
  kind: "app",
  checkout: { kind: "job", title: "Pin the store clock", branch: "fix/pin-store-clock", bay: 4 },
  processes: [
    { id: "bridge", name: "bridge", port: 5390, phase: "ready", url: "http://localhost:5390", log: ["electron launched"] },
    { id: "renderer", name: "renderer", phase: "starting", log: ["compiling"] },
  ],
};

/**
 * **Main, a row for each process, and Open only where there is something to open.** Each row has the state mark and the leased port, and a process with no port leaves
 * the column empty. Open is on a ready process that has an address and nowhere else: not on one still starting, one that exited, or one with no address. Each
 * process has a tab, and the log under it is that process's.
 */
export const BrowserOnMain: Story = {
  args: { app: BROWSER, onOpen: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("On")).toBeVisible();
    await expect(canvas.getByText("main")).toBeVisible();

    const rows = within(canvas.getByRole("list", { name: "Processes" }));
    const row = (name: string) => within(rows.getByRole("listitem", { name }));
    await expect(row("backend").getByRole("img", { name: "Ready" })).toBeVisible();
    await expect(row("backend").getByText("8001")).toBeVisible();
    await expect(row("frontend").getByRole("img", { name: "Starting" })).toBeVisible();
    await expect(row("frontend").getByText("5173")).toBeVisible();
    await expect(row("worker").getByRole("img", { name: "Ready" })).toBeVisible();
    await expect(row("legacy").getByRole("img", { name: "Exited" })).toBeVisible();
    await expect(row("worker").queryByText(/^\d+$/)).toBeNull();

    await expect(canvas.getAllByRole("button", { name: /^Open / })).toHaveLength(1);
    await expect(row("backend").getByRole("button", { name: "Open backend" })).toBeVisible();
    await expect(row("frontend").queryByRole("button")).toBeNull();
    await expect(row("worker").queryByRole("button")).toBeNull();
    await expect(row("legacy").queryByRole("button")).toBeNull();
    await userEvent.click(row("backend").getByRole("button", { name: "Open backend" }));
    await expect(args.onOpen).toHaveBeenCalledWith("http://localhost:8001");

    await expect(canvas.getAllByRole("tab").map((one) => one.textContent)).toEqual(["backend", "frontend", "worker", "legacy"]);
    await expect(canvas.getByRole("tab", { name: "backend" })).toHaveAttribute("aria-selected", "true");
    await expect(within(canvas.getByRole("region", { name: "backend log" })).getByText("INFO listening on 8001")).toBeVisible();
    await userEvent.click(canvas.getByRole("tab", { name: "frontend" }));
    await expect(canvas.getByRole("tab", { name: "frontend" })).toHaveAttribute("aria-selected", "true");
    const log = within(canvas.getByRole("region", { name: "frontend log" }));
    await expect(log.getByText("VITE v6 ready")).toBeVisible();
    await expect(log.queryByText("INFO listening on 8001")).toBeNull();
  },
};

/**
 * **A Job's bay, and a desktop app that has no address.** The panel names the Job with its branch and bay, not main. A desktop-app run shows the window mark where a browser
 * run has Open, even for a process that was given an address, and says whether the window is open or still opening.
 */
export const AppOnAJob: Story = {
  args: { app: APP, onOpen: fn() },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Job")).toBeVisible();
    await expect(canvas.getByText("Pin the store clock")).toBeVisible();
    await expect(canvas.getByText("fix/pin-store-clock")).toBeVisible();
    await expect(canvas.getByText("Bay")).toBeVisible();
    await expect(canvas.getByText("4")).toBeVisible();
    await expect(canvas.queryByText("main")).toBeNull();

    const rows = within(canvas.getByRole("list", { name: "Processes" }));
    await expect(within(rows.getByRole("listitem", { name: "bridge" })).getByRole("img", { name: "Window open" })).toBeVisible();
    await expect(within(rows.getByRole("listitem", { name: "renderer" })).getByRole("img", { name: "Window opening" })).toBeVisible();
    await expect(within(rows.getByRole("listitem", { name: "bridge" })).getByText("5390")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /^Open / })).toBeNull();
    await expect(canvas.getAllByRole("tab")).toHaveLength(2);
  },
};
