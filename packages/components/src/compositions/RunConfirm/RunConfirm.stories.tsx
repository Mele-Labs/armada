import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { forgetAllKept } from "../../keep";
import type { RunDraft } from "../RunControl/runs";
import { RunConfirm } from "./RunConfirm";

/**
 * The confirm a repository with no saved run is asked first: the run Fleet drafted, each process's name, folder and command editable, and where it
 * saves. One act, Save and run, hands back what is on screen.
 */
const meta: Meta<typeof RunConfirm> = {
  title: "Compositions/Run confirm",
  component: RunConfirm,
  // What a story typed is kept under its `keep`; the next starts from a window that kept nothing.
  beforeEach: () => forgetAllKept(),
};
export default meta;

type Story = StoryObj<typeof RunConfirm>;

const DRAFT: RunDraft = {
  root: "~/Development/auto-ynab",
  savedIn: "this Mac",
  processes: [
    { id: "backend", name: "backend", folder: ".", command: "uvicorn backend.app.main:app --port ${port}" },
    { id: "frontend", name: "frontend", folder: "frontend", command: "npx vite --port ${port} --strictPort" },
  ],
};

const args = { open: true, repository: "auto-ynab", draft: DRAFT, onSave: fn(), onCancel: fn() } as const;

/**
 * **A folder is read from the root, and the caption says where that is.** The folder's full path sits under the row and follows each keystroke, with `.` and an empty
 * field showing the root itself. A command that asks for `${port}` says it is leased, naming the variable the other processes read it by; one that does not says nothing
 * there. Where it saves is a fact under the processes.
 */
export const Editing: Story = {
  args: { ...args, keep: "editing" },
  play: async ({ canvas, userEvent }) => {
    const dialog = within(canvas.getByRole("dialog", { name: "Run auto-ynab" }));
    await expect(dialog.getByLabelText("Process 1 name")).toHaveFocus();
    await expect(dialog.getAllByLabelText(/^Process \d name$/).map((one) => (one as HTMLInputElement).value)).toEqual(["backend", "frontend"]);

    const row = (at: number) => within(dialog.getAllByRole("listitem")[at - 1]!);
    await expect(row(1).getByText("~/Development/auto-ynab", { selector: "code" })).toBeVisible();
    await expect(row(2).getByText("~/Development/auto-ynab/frontend", { selector: "code" })).toBeVisible();

    await userEvent.clear(dialog.getByLabelText("Process 2 folder"));
    await expect(row(2).getByText("~/Development/auto-ynab", { selector: "code" })).toBeVisible();
    await userEvent.type(dialog.getByLabelText("Process 2 folder"), "web/app");
    await expect(row(2).getByText("~/Development/auto-ynab/web/app", { selector: "code" })).toBeVisible();

    await expect(row(1).getByText("leased")).toHaveAccessibleDescription("The other processes read it as ARMADA_PORT_BACKEND");
    await expect(row(2).getByText("leased")).toHaveAccessibleDescription("The other processes read it as ARMADA_PORT_FRONTEND");
    await userEvent.clear(dialog.getByLabelText("Process 2 command"));
    await userEvent.type(dialog.getByLabelText("Process 2 command"), "pnpm test:watch");
    await expect(row(2).queryByText("leased")).toBeNull();
    await expect(row(1).getByText("leased")).toBeVisible();

    await expect(dialog.getByText("Saved in")).toBeVisible();
    await expect(dialog.getByText("this Mac", { selector: "code" })).toBeVisible();
  },
};

/** **Save and run hands back what was edited**, the folder trimmed and an empty one read as the root, and nothing else is saved before it. */
export const SaveAndRun: Story = {
  args: { ...args, onSave: fn(), keep: "save" },
  play: async ({ args: given, canvas, userEvent }) => {
    const dialog = within(canvas.getByRole("dialog", { name: "Run auto-ynab" }));
    await userEvent.clear(dialog.getByLabelText("Process 1 name"));
    await userEvent.type(dialog.getByLabelText("Process 1 name"), "api");
    await userEvent.clear(dialog.getByLabelText("Process 1 folder"));
    await userEvent.clear(dialog.getByLabelText("Process 2 folder"));
    await userEvent.type(dialog.getByLabelText("Process 2 folder"), "  web  ");
    await userEvent.clear(dialog.getByLabelText("Process 2 command"));
    await userEvent.type(dialog.getByLabelText("Process 2 command"), "pnpm dev");
    await expect(given.onSave).not.toHaveBeenCalled();

    await userEvent.click(dialog.getByRole("button", { name: /Save and run/ }));
    await expect(given.onSave).toHaveBeenCalledTimes(1);
    await expect(given.onSave).toHaveBeenCalledWith(
      [
        { id: "backend", name: "api", folder: ".", command: "uvicorn backend.app.main:app --port ${port}" },
        { id: "frontend", name: "frontend", folder: "web", command: "pnpm dev" },
      ],
      "",
      [],
    );
  },
};

/** **Enter in a field does what Save and run does**, because the confirm collects; Esc cancels and saves nothing. */
export const EnterSaves: Story = {
  args: { ...args, onSave: fn(), onCancel: fn(), keep: "enter" },
  play: async ({ args: given, canvas, userEvent }) => {
    const dialog = within(canvas.getByRole("dialog", { name: "Run auto-ynab" }));
    await userEvent.type(dialog.getByLabelText("Process 1 name"), "2");
    await userEvent.keyboard("{Enter}");
    await expect(given.onSave).toHaveBeenCalledTimes(1);
    await expect(given.onSave).toHaveBeenCalledWith(
      [
        { id: "backend", name: "backend2", folder: ".", command: "uvicorn backend.app.main:app --port ${port}" },
        { id: "frontend", name: "frontend", folder: "frontend", command: "npx vite --port ${port} --strictPort" },
      ],
      "",
      [],
    );
    await expect(given.onCancel).not.toHaveBeenCalled();

    await userEvent.keyboard("{Escape}");
    await expect(given.onCancel).toHaveBeenCalledTimes(1);
    await expect(given.onSave).toHaveBeenCalledTimes(1);
  },
};

/**
 * **A process is added or removed in the list.** Add process appends an empty row, and Save and run stays off until it has a name and a command; removing a row
 * takes it out of what is saved; a run with no process left cannot be saved.
 */
export const AddAndRemove: Story = {
  args: { ...args, onSave: fn(), keep: "add-remove" },
  play: async ({ args: given, canvas, userEvent }) => {
    const dialog = within(canvas.getByRole("dialog", { name: "Run auto-ynab" }));
    const save = () => dialog.getByRole("button", { name: /Save and run/ });
    await expect(save()).toBeEnabled();

    await userEvent.click(dialog.getByRole("button", { name: "Add process" }));
    await expect(dialog.getAllByRole("listitem")).toHaveLength(3);
    await expect(dialog.getByLabelText("Process 3 name")).toHaveValue("");
    await expect(dialog.getByLabelText("Process 3 folder")).toHaveValue(".");
    await expect(save()).toBeDisabled();

    await userEvent.type(dialog.getByLabelText("Process 3 name"), "worker");
    await expect(save()).toBeDisabled();
    await userEvent.type(dialog.getByLabelText("Process 3 command"), "node worker.js");
    await expect(save()).toBeEnabled();
    await expect(within(dialog.getAllByRole("listitem")[2]!).queryByText("leased")).toBeNull();

    await userEvent.click(dialog.getByRole("button", { name: "Remove process 1" }));
    await expect(dialog.getAllByRole("listitem")).toHaveLength(2);
    await expect(dialog.getAllByLabelText(/^Process \d name$/).map((one) => (one as HTMLInputElement).value)).toEqual(["frontend", "worker"]);

    await userEvent.click(save());
    await expect(given.onSave).toHaveBeenCalledWith(
      [
        { id: "frontend", name: "frontend", folder: "frontend", command: "npx vite --port ${port} --strictPort" },
        expect.objectContaining({ name: "worker", folder: ".", command: "node worker.js" }),
      ],
      "",
      [],
    );
  },
};

/** **Nothing left to run.** Removing every process leaves Save and run off. */
export const NoProcesses: Story = {
  args: { ...args, keep: "none" },
  play: async ({ canvas, userEvent }) => {
    const dialog = within(canvas.getByRole("dialog", { name: "Run auto-ynab" }));
    await userEvent.click(dialog.getByRole("button", { name: "Remove process 1" }));
    await userEvent.click(dialog.getByRole("button", { name: "Remove process 1" }));
    await expect(dialog.queryAllByRole("listitem")).toHaveLength(0);
    await expect(dialog.getByRole("button", { name: /Save and run/ })).toBeDisabled();
  },
};
