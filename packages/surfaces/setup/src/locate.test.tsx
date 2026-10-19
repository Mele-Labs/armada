// Locate, through `App` — Journey 3's *Getting in*, from the picker's own Add a
// repository: a folder added, or a clone from a URL, landing on the Cockpit with
// it picked; Setup opens only when asked for. Moved
// here from `Screens/Locate`'s stories — #1224. Nothing native opens: the
// folder dialog answers `CHOSEN`.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { MANIFEST_NAME, repository } from "@armada/screens/src/fixtures/repository";

import { CHOSEN, entered, mount, mountTwo, settingUp, unmountAfterEach } from "@armada/desktop/mock";
import type { SettingUp } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Setup only: what the dialog and the notice read is its own slice's.
const SLICES = { slices: ["core", "setup"] } as const;

const URL = "https://forge.invalid/owner/storefront.git";
const dialog = () => page.getByRole("dialog", { name: "Add a repository" });

/** The repository picker on the Cockpit: a combobox labelled Repository, whose value is the name of the one chosen. */
const repositoryPicker = (within: ReturnType<typeof page.elementLocator> | typeof page = page, options: { includeHidden?: boolean } = {}) =>
  within.getByRole("combobox", { name: "Repository", exact: true, ...options });

/** Add a repository is the last row of the deck's repository picker: opening the dialog is opening the picker first. */
async function opened(within: ReturnType<typeof page.elementLocator> | typeof page = page) {
  await within.getByRole("combobox", { name: "Repository", exact: true }).click();
  await page.getByRole("option", { name: "Add a repository" }).click();
  await entered(dialog());
  return dialog();
}

/** Where nothing is served the dialog opens by itself, and there is no picker to press. */
async function openedByItself() {
  await entered(dialog());
  return dialog();
}

async function cloneFrom(within: ReturnType<typeof page.elementLocator> | typeof page = page, open: typeof opened = opened) {
  const add = await open(within);
  await add.getByRole("button", { name: "Clone from a URL" }).click();
  await userEvent.type(add.getByLabelText("Repository URL"), URL);
  await userEvent.type(add.getByLabelText("Clone into"), "/Users/user/code");
  // Named before anything is pressed, as Fleet will name it.
  await expect.element(add.getByRole("group", { name: "Project location" }).getByText("/Users/user/code/storefront")).toBeVisible();
  await add.getByRole("button", { name: /^Clone repository/ }).click();
  return add;
}

const locating = (options: SettingUp = {}) => mount(settingUp(options), SLICES);

/** Setup asked for, by the control beside the picker and the Manifest's Set up workspaces tab. */
async function askForSetup() {
  await page.getByRole("button", { name: "Open the Manifest", exact: true }).click();
  await page.getByRole("tab", { name: "Set up workspaces" }).click();
  await expect.element(page.getByRole("region", { name: "Workspaces" })).toBeVisible();
}

test("a folder added: the dialog closes, the Cockpit holds it in the picker, and Setup opens only when asked for", async () => {
  const onAdded = vi.fn();
  locating({ onAdded });
  const add = await opened();
  await add.getByRole("button", { name: "Choose a folder" }).click();
  await expect.element(add.getByLabelText("Project location")).toHaveValue(CHOSEN);
  await add.getByRole("button", { name: /^Add repository/ }).click();
  expect(onAdded).toHaveBeenCalledWith(CHOSEN);
  await expect.poll(() => dialog().query()).toBeNull();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  // Nobody's Manifest yet, so it reads by its own name. The picker is on the Cockpit, which the add landed on.
  await repositoryPicker().click();
  const menu = page.getByRole("listbox", { name: "Repositories" });
  await expect.element(menu.getByRole("option", { name: "scratch" })).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  await askForSetup();
});

test("nothing served: the dialog opens by itself, nothing reads as a fault, and the first add lands on the Cockpit", async () => {
  const onAdded = vi.fn();
  locating({ repositories: [], onAdded });
  const add = dialog();
  await entered(add);
  expect(page.getByText(/could not be read/).query()).toBeNull();
  await userEvent.type(add.getByLabelText("Project location"), CHOSEN);
  await add.getByRole("button", { name: /^Add repository/ }).click();
  expect(onAdded).toHaveBeenCalledWith(CHOSEN);
  await expect.element(repositoryPicker()).toHaveValue("scratch");
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
});

test("a clone that finished after its dialog closed says so, and moves nothing until Open picks it on the Cockpit", async () => {
  locating({ clone: "late" });
  const add = await cloneFrom();
  await add.getByRole("button", { name: /^Cancel/ }).click();
  await expect.poll(() => dialog().query()).toBeNull();
  await expect.element(page.getByText("storefront is ready")).toBeVisible();
  await expect.element(repositoryPicker()).toHaveValue(MANIFEST_NAME);
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  await page.getByRole("button", { name: "Open", exact: true }).click();
  await expect.element(repositoryPicker()).toHaveValue("storefront");
  await expect.poll(() => page.getByText("storefront is ready").query()).toBeNull();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  await askForSetup();
});

test("a late clone is heard in another window on the same main, and neither window's pick moves until Open", async () => {
  const [asked, other] = mountTwo(settingUp({ clone: "late" }), ["The window that asked", "Another window"], SLICES);
  const inAsked = page.elementLocator(asked);
  const inOther = page.elementLocator(other);
  const add = await cloneFrom(inAsked);
  await add.getByRole("button", { name: /^Cancel/ }).click();
  // Present rather than visible: two windows in one test frame are each too narrow to lay the notice out.
  await expect.element(inOther.getByText("storefront is ready")).toBeInTheDocument();
  await expect.element(inAsked.getByText("storefront is ready")).toBeInTheDocument();
  for (const window of [inAsked, inOther]) {
    // Hidden, not absent: each window is narrow, and the picker is behind its menu.
    await expect.element(repositoryPicker(window, { includeHidden: true })).toHaveValue(MANIFEST_NAME);
    expect(window.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  }
  (inOther.getByRole("button", { name: "Open", exact: true }).element() as HTMLElement).click();
  await expect.element(repositoryPicker(inOther, { includeHidden: true })).toHaveValue("storefront");
  expect(inOther.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  (inAsked.getByRole("button", { name: "Dismiss" }).element() as HTMLElement).click();
  await expect.poll(() => inAsked.getByText("storefront is ready").query()).toBeNull();
});

test("a parent under a symlink previews the folder Fleet clones into, as main resolves it", async () => {
  locating({ repositories: [] });
  const add = await openedByItself();
  await add.getByRole("button", { name: "Clone from a URL" }).click();
  await userEvent.type(add.getByLabelText("Repository URL"), URL);
  await userEvent.type(add.getByLabelText("Clone into"), "/tmp");
  await expect.element(add.getByRole("group", { name: "Project location" }).getByText("/private/tmp/storefront")).toBeVisible();
  await add.getByRole("button", { name: /^Cancel/ }).click();
});

test("a clone underway says so, and a second press — the button or Enter — sends nothing", async () => {
  const onCloned = vi.fn();
  locating({ repositories: [], clone: "underway", onCloned });
  const add = await cloneFrom(page, openedByItself);
  await expect.element(add.getByText(/Git is cloning into/)).toBeVisible();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(onCloned).toHaveBeenCalledTimes(1);
  expect(onCloned).toHaveBeenCalledWith(URL, "/Users/user/code");
});

test("git's refusal reads in the dialog in full, and nothing moves behind it", async () => {
  locating({ repositories: [], clone: "refused" });
  const add = await cloneFrom(page, openedByItself);
  await expect.element(add.getByText("Not cloned")).toBeVisible();
  await expect.element(add.getByText(`git refused the clone: fatal: repository '${URL}' not found.`)).toBeVisible();
  expect(add.getByText("fleet.clone_refused").query()).toBeNull();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeEnabled();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
});

test("a destination already full: Fleet's words, and where to go from them", async () => {
  locating({ repositories: [], clone: "occupied" });
  const add = await cloneFrom(page, openedByItself);
  await expect.element(add.getByText("Not cloned")).toBeVisible();
  await expect.element(add.getByText("/Users/user/code/storefront already exists and is not empty.")).toBeVisible();
  await expect.element(add.getByText("Choose another folder to clone into.")).toBeVisible();
  expect(add.getByRole("group", { name: "Project location" }).query()).toBeNull();
  expect(add.getByText("fleet.destination_occupied").query()).toBeNull();
});

test("Escape closes it with nothing sent, and it opens again empty", async () => {
  const onAdded = vi.fn();
  locating({ onAdded });
  const add = await opened();
  await userEvent.type(add.getByLabelText("Project location"), "/Users/user/elsewhere");
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => dialog().query()).toBeNull();
  expect(onAdded).not.toHaveBeenCalled();
  const again = await opened();
  await expect.element(again.getByLabelText("Project location")).toHaveValue("");
  await again.getByRole("button", { name: /^Cancel/ }).click();
  await expect.poll(() => dialog().query()).toBeNull();
});

/**
 * A set-up repository, named by what Fleet read its Manifest under. **The id
 * stays a ULID**, which is what `ManifestId` is — so a fixture that put a
 * readable word there was testing a label that could never appear.
 */
const named = (name: string, root: string): RepositorySummary => ({
  ...repository(),
  root,
  manifest: { ...repository().manifest!, repository: name },
});

const SET_UP = named("storefront", "/Users/user/code/web-app");
const LOOSE: RepositorySummary = { root: "/Users/user/scratch", records_root: "/records/scratch" };
const API: RepositorySummary = { root: "/Users/user/code/api", records_root: "/records/api" };
const OLD_API: RepositorySummary = { root: "/Users/user/old/api", records_root: "/records/old-api" };
const SET_UP_API = named("api", "/Users/user/services/api");

test("the picker's names: the Manifest's own, or a folder's, with the parent only where two share it", async () => {
  locating({ repositories: [SET_UP, SET_UP_API, LOOSE, API, OLD_API] });
  await repositoryPicker().click();
  await expect.element(page.getByRole("option", { name: "storefront" })).toBeInTheDocument();
  // Three repositories would read "api" — one set up and two not — so all three
  // widen. The ladder is one namespace since 28 Sep 2026: a set-up repository
  // used to be exempt because its label was its unique id, and its label is a
  // name now, which can collide like any other.
  await expect.element(page.getByRole("option", { name: "services/api" })).toBeInTheDocument();
  expect(page.getByRole("option", { name: "api", exact: true }).query()).toBeNull();
  await expect.element(page.getByRole("option", { name: "scratch" })).toBeInTheDocument();
  await expect.element(page.getByRole("option", { name: "code/api" })).toBeInTheDocument();
  await expect.element(page.getByRole("option", { name: "old/api" })).toBeInTheDocument();
  expect(page.getByRole("option", { name: "web-app" }).query()).toBeNull();
});
