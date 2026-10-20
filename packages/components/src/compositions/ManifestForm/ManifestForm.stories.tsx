import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { PrOwnerRegistryProvider } from "../RegisteredOwner/RegisteredOwner";
import { ManifestForm, type ManifestFormDraft } from "./ManifestForm";

/**
 * The Manifest as forms — Journey 9, *Editing* — drawn from lines in this
 * repository's own `armada.yml`, in the shape the Manifest surface holds a
 * draft.
 *
 * **The policy words are Fleet's**, handed across with the value, so nothing
 * here spells a registry of its own.
 */
const meta: Meta<typeof ManifestForm> = {
  title: "Compositions/Manifest form",
  component: ManifestForm,
};
export default meta;

type Story = StoryObj<typeof ManifestForm>;

const noop = () => {};

const DRAFT: ManifestFormDraft = {
  checks: [
    {
      name: "build",
      run: "cargo build --workspace --locked",
      expectExitCode: "",
      requires: [],
      when: "",
      narrow: { run: "cargo build --locked", each: "-p {}", from: "", under: "crates", except: "" },
    },
    { name: "bridge_test", run: "pnpm bridge-test", expectExitCode: "", requires: ["bootstrap"], when: "packages/**\napps/**", narrow: null },
  ],
  commands: [
    { name: "bootstrap", run: "pnpm install --frozen-lockfile", destructive: false, serve: "", ready: "", links: [] },
    {
      name: "storybook_dev",
      run: "",
      destructive: false,
      serve: "pnpm -C packages/components storybook --port ${port.storybook}",
      ready: "curl -sf http://localhost:${port.storybook}",
      links: [{ url: "http://localhost:${port.storybook}", name: "Storybook" }],
    },
  ],
  ports: [{ name: "storybook", container: "", env: "STORYBOOK_PORT" }],
  freeze: false,
  autoMerge: "never",
  reviewGate: "human_always",
  costCap: "5",
  turnCap: "300",
  base: "main",
  evidence: {
    serve: "pnpm -C packages/components storybook --port ${port.storybook}",
    ready: "curl -sf http://localhost:${port.storybook}",
    run: "pnpm -C packages/components exec playwright test {}",
    frames: ".armada/frames",
    never: "/__notes",
  },
  afterMerge: ["build"],
  setup: ["bootstrap"],
  quietAfter: "300",
  pokeLimit: "",
  excludePaths: "",
  draftPr: false,
};

const BASE = {
  path: "/Users/user/armada/armada.yml",
  identity: { id: "armada", version: 1 },
  draft: DRAFT,
  onDraft: noop,
  autoMergeWords: ["never", "checks-pass", "always"],
  reviewGateWords: ["human_always", "auto_if_judge_passes"],
  problems: {},
  changed: false,
  onSave: noop,
  onDiscard: noop,
};

/** Nothing touched: Save and Discard wait for a change. */
export const AtRest: Story = {
  name: "At rest",
  args: BASE,
  // Fleet serves no such key, so without the registry the form draws no switch for it.
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("switch", { name: "Pull request owners" })).toBeNull();
  },
};

/** A change not yet saved. */
export const Changed: Story = {
  name: "Changed",
  args: { ...BASE, changed: true },
};

/** Fleet refused a result that would not load, and said which key. The draft stays. */
export const Refused: Story = {
  name: "Refused",
  args: {
    ...BASE,
    changed: true,
    refused: {
      saying: "armada.yml would not load after these edits: 1 fault",
      faults: [{ key: "checks.bridge_test.requires", fault: "names browsers, which no Command declares" }],
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("checks.bridge_test.requires")).toBeVisible();
  },
};

/** A window that holds the owner registry draws the switch in Policy, off until the repository turns it on. */
export const RegisteringOwners: Story = {
  name: "Registering pull request owners",
  args: BASE,
  decorators: [
    (Story) => (
      <PrOwnerRegistryProvider value={{ ownerOf: () => undefined, registering: true, setRegistering: noop }}>
        <Story />
      </PrOwnerRegistryProvider>
    ),
  ],
  play: async ({ canvasElement }) => {
    const policy = within(within(canvasElement).getByRole("region", { name: "Policy" }));
    await expect(policy.getByRole("switch", { name: "Pull request owners" })).toBeChecked();
  },
};

/** Frozen: the switch reads what the file declares, and turning it off is one change to save. */
export const Frozen: Story = {
  name: "Frozen",
  args: { ...BASE, draft: { ...DRAFT, freeze: true } },
  play: async ({ canvasElement }) => {
    const freeze = within(within(canvasElement).getByRole("region", { name: "Freeze" }));
    await expect(freeze.getByRole("switch", { name: /Freeze this repository/ })).toBeChecked();
  },
};
