// Locate, through `App` — Journey 3's *Getting in*, from the picker's own Add a
// repository: a folder added, or a clone from a URL, landing on the Cockpit with
// it picked; or from New project above it, a folder made under a location; Setup opens only when asked for. Moved
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

/** The repository picker on the Cockpit: a button labelled Repository, whose text is the name of the one chosen. */
const repositoryPicker = (within: ReturnType<typeof page.elementLocator> | typeof page = page, options: { includeHidden?: boolean } = {}) =>
  within.getByRole("button", { name: "Repository", exact: true, ...options });

/** Add a repository is in the footer of the title row's repository picker: opening the dialog is opening the picker first. */
async function opened(within: ReturnType<typeof page.elementLocator> | typeof page = page) {
  await within.getByRole("button", { name: "Repository", exact: true }).click();
  await page.getByRole("menuitem", { name: "Add a repository" }).click();
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
  const menu = page.getByRole("menu");
  await expect.element(menu.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  await askForSetup();
});

test("nothing served: the start menu stands in for the dialog, and nothing reads as a fault", async () => {
  locating({ repositories: [] });
  await expect.element(page.getByRole("group", { name: "Start from" }).getByRole("button", { name: "Open a project" })).toBeVisible();
  expect(dialog().query()).toBeNull();
  expect(page.getByText(/could not be read/).query()).toBeNull();
});

test("a clone that finished after its dialog closed says so, and moves nothing until Open picks it on the Cockpit", async () => {
  locating({ clone: "late" });
  const add = await cloneFrom();
  await add.getByRole("button", { name: /^Cancel/ }).click();
  await expect.poll(() => dialog().query()).toBeNull();
  await expect.element(page.getByText("storefront is ready")).toBeVisible();
  await expect.element(repositoryPicker()).toHaveTextContent(MANIFEST_NAME);
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  await page.getByRole("button", { name: "Open", exact: true }).click();
  await expect.element(repositoryPicker()).toHaveTextContent("storefront");
  await expect.poll(() => page.getByText("storefront is ready").query()).toBeNull();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  await askForSetup();
});

test("a late clone is heard in another window on the same main, and neither window's pick moves until Open", async () => {
  const [asked, other] = mountTwo(settingUp({ clone: "late" }), ["The window that asked", "Another window"], SLICES);
  // One above the other, each the window's full width: side by side the title row has no room left for the picker.
  asked.parentElement!.style.flexDirection = "column";
  const inAsked = page.elementLocator(asked);
  const inOther = page.elementLocator(other);
  const add = await cloneFrom(inAsked);
  await add.getByRole("button", { name: /^Cancel/ }).click();
  // Present rather than visible: two windows in one test frame are each too narrow to lay the notice out.
  await expect.element(inOther.getByText("storefront is ready")).toBeInTheDocument();
  await expect.element(inAsked.getByText("storefront is ready")).toBeInTheDocument();
  for (const window of [inAsked, inOther]) {
    // Hidden, not absent: each window is narrow, and the picker is behind its menu.
    await expect.element(repositoryPicker(window, { includeHidden: true })).toHaveTextContent(MANIFEST_NAME);
    expect(window.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  }
  (inOther.getByRole("button", { name: "Open", exact: true }).element() as HTMLElement).click();
  await expect.element(repositoryPicker(inOther, { includeHidden: true })).toHaveTextContent("storefront");
  expect(inOther.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  (inAsked.getByRole("button", { name: "Dismiss" }).element() as HTMLElement).click();
  await expect.poll(() => inAsked.getByText("storefront is ready").query()).toBeNull();
});

test("a parent under a symlink previews the folder Fleet clones into, as main resolves it", async () => {
  locating();
  const add = await opened();
  await add.getByRole("button", { name: "Clone from a URL" }).click();
  await userEvent.type(add.getByLabelText("Repository URL"), URL);
  await userEvent.type(add.getByLabelText("Clone into"), "/tmp");
  await expect.element(add.getByRole("group", { name: "Project location" }).getByText("/private/tmp/storefront")).toBeVisible();
  await add.getByRole("button", { name: /^Cancel/ }).click();
});

test("a clone underway says so, and a second press — the button or Enter — sends nothing", async () => {
  const onCloned = vi.fn();
  locating({ clone: "underway", onCloned });
  const add = await cloneFrom();
  await expect.element(add.getByText(/Git is cloning into/)).toBeVisible();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(onCloned).toHaveBeenCalledTimes(1);
  expect(onCloned).toHaveBeenCalledWith(URL, "/Users/user/code");
});

test("git's refusal reads in the dialog in full, and nothing moves behind it", async () => {
  locating({ clone: "refused" });
  const add = await cloneFrom();
  await expect.element(add.getByText("Not cloned")).toBeVisible();
  await expect.element(add.getByText(`git refused the clone: fatal: repository '${URL}' not found.`)).toBeVisible();
  expect(add.getByText("fleet.clone_refused").query()).toBeNull();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeEnabled();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
});

test("a destination already full: Fleet's words, and where to go from them", async () => {
  locating({ clone: "occupied" });
  const add = await cloneFrom();
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

const created = () => page.getByRole("dialog", { name: "New project" });

/** New project is the footer row above Add a repository in the title row's picker: the same dialog, on a new project. */
async function newProject() {
  await repositoryPicker().click();
  await page.getByRole("menuitem", { name: "New project" }).click();
  await entered(created());
  return created();
}

/** A name typed and Create project pressed. */
async function make(name = "my-app") {
  const dialog = await newProject();
  await userEvent.type(dialog.getByLabelText("Name"), name);
  await dialog.getByRole("button", { name: /^Create project/ }).click();
  return dialog;
}

test("a new project: offered where the projects are, made under it, and listed in the picker", async () => {
  const onCreated = vi.fn();
  locating({ repositories: [SET_UP, API], onCreated });
  const dialog = await newProject();
  await expect.element(dialog.getByLabelText("Location")).toHaveValue("/Users/user/code");
  await expect.element(dialog.getByRole("button", { name: /^Create project/ })).toBeDisabled();
  await userEvent.type(dialog.getByLabelText("Name"), "my-app");
  await dialog.getByRole("button", { name: /^Create project/ }).click();
  expect(onCreated).toHaveBeenCalledWith("/Users/user/code", "my-app");
  await expect.poll(() => created().query()).toBeNull();
  await repositoryPicker().click();
  await expect.element(page.getByRole("menu").getByRole("menuitem", { name: "my-app" })).toBeInTheDocument();
});

test("a name that is not one folder's is said under the field, and nothing is sent", async () => {
  const onCreated = vi.fn();
  locating({ repositories: [SET_UP, API], onCreated });
  const dialog = await newProject();
  await userEvent.type(dialog.getByLabelText("Name"), "my app");
  await expect.element(dialog.getByText("Letters, digits, dots, dashes and underscores.")).toBeVisible();
  await expect.element(dialog.getByRole("button", { name: /^Create project/ })).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(onCreated).not.toHaveBeenCalled();
});

test("a new project's folder already full: Fleet's words, and where to go from them", async () => {
  locating({ repositories: [SET_UP, API], create: "occupied" });
  const dialog = await make();
  await expect.element(dialog.getByText("Not made")).toBeVisible();
  await expect.element(dialog.getByText("/Users/user/code/my-app already exists and is not empty.")).toBeVisible();
  await expect.element(dialog.getByText("Choose another name or location.")).toBeVisible();
  await expect.element(dialog.getByRole("button", { name: /^Create project/ })).toBeEnabled();
});

test("a new project made and then not served says the folder stays", async () => {
  locating({ repositories: [SET_UP, API], create: "unserved" });
  const dialog = await make();
  await expect.element(dialog.getByText("Made, and not added")).toBeVisible();
  await expect.element(dialog.getByText("/Users/user/code/my-app could not be read as a repository.")).toBeVisible();
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
  await expect.element(page.getByRole("menuitem", { name: "storefront" })).toBeInTheDocument();
  // Three repositories would read "api" — one set up and two not — so all three
  // widen. The ladder is one namespace since 28 Sep 2026: a set-up repository
  // used to be exempt because its label was its unique id, and its label is a
  // name now, which can collide like any other.
  await expect.element(page.getByRole("menuitem", { name: "services/api" })).toBeInTheDocument();
  expect(page.getByRole("menuitem", { name: "api", exact: true }).query()).toBeNull();
  await expect.element(page.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  await expect.element(page.getByRole("menuitem", { name: "code/api" })).toBeInTheDocument();
  await expect.element(page.getByRole("menuitem", { name: "old/api" })).toBeInTheDocument();
  expect(page.getByRole("menuitem", { name: "web-app" }).query()).toBeNull();
});
