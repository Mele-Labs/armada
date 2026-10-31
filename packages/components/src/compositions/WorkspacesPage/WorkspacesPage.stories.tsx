import type { Meta, StoryObj } from "@storybook/react-vite";
import { WorkspacesPage } from "./WorkspacesPage";

const repositories = [
  { id: "store-web", name: "store-web" },
  { id: "store-api", name: "store-api" },
  { id: "store-docs", name: "store-docs" },
  { id: "scratch", name: "scratch" },
];

const workspaces = [
  { id: "my-store", name: "my-store", repositories: ["store-web", "store-api"] },
  { id: "platform", name: "platform", repositories: ["store-docs", "store-api"], home: "store-docs" },
];

const layers = {
  everywhere: { start_job: "asks" as const },
  "workspace:my-store": { tracker_write: "off" as const, run_command: "acts" as const },
  "workspace:platform": { run_command: "asks" as const },
};

const meta: Meta<typeof WorkspacesPage> = {
  title: "Compositions/Workspaces page",
  component: WorkspacesPage,
  args: {
    repositories,
    workspaces,
    layers,
    onPick: () => {},
    onSet: () => {},
    onCreate: () => {},
    onSave: () => {},
    onRemove: () => {},
    onOpenWorkflows: () => {},
  },
};
export default meta;

type Story = StoryObj<typeof WorkspacesPage>;

/** A Workspace picked: its name, its repositories and what it sets of each kind of act. */
export const AWorkspace: Story = {
  args: { picked: { kind: "workspace", id: "my-store" } },
};

/** A repository in two Workspaces that set Commands differently. */
export const ARepositoryInTwoWorkspaces: Story = {
  args: { picked: { kind: "repository", id: "store-api" } },
};

/** Everywhere, the level under every Workspace and repository. */
export const Everywhere: Story = {
  args: { picked: { kind: "everywhere" } },
};

/** The empty form a new Workspace starts from. */
export const ANewWorkspace: Story = {
  args: { picked: { kind: "new" } },
};

/** Fleet refused a save, and says why where the press was. */
export const ARefusedSave: Story = {
  args: { picked: { kind: "workspace", id: "platform" }, notice: "platform lives in store-docs, so it is changed there." },
};
