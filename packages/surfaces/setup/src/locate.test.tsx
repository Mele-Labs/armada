// Locate, through `App` — Journey 3's *Getting in*, from the picker's own Add a
// repository: a folder added, or a clone from a URL, landing on Setup. Moved
// here from `Screens/Locate`'s stories — #1224. Nothing native opens: the
// folder dialog answers `CHOSEN`.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { MANIFEST_NAME, repository } from "@armada/screens/src/fixtures/build/base";

import { CHOSEN, entered, mount, mountTwo, settingUp, toPlace, unmountAfterEach } from "@armada/desktop/mock";
import type { SettingUp } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Setup only: what the dialog and the notice read is its own slice's.
const SLICES = { slices: ["core", "setup"] } as const;

const URL = "https://forge.invalid/owner/storefront.git";
const dialog = () => page.getByRole("dialog", { name: "Add a repository" });

/** The repository picker on the Cockpit: labelled Repository, and its text is the name of the one chosen. */
const repositoryPicker = (within: ReturnType<typeof page.elementLocator> | typeof page = page, options: { includeHidden?: boolean } = {}) =>
  within.getByRole("button", { name: "Repository", exact: true, ...options });

/**
 * The dialog, already open. **Add a repository has no row on the flight deck's picker**, and the title menu draws the
 * picker's rows only where its container is narrow, which its portalled panel does not see; the one way in left is a
 * Fleet that serves no repository, where the dialog opens by itself.
 */
async function opened() {
  await entered(dialog());
  return dialog();
}

async function cloneFrom(_within: ReturnType<typeof page.elementLocator> | typeof page = page) {
  const add = await opened();
  await add.getByRole("button", { name: "Clone from a URL" }).click();
  await userEvent.type(add.getByLabelText("Repository URL"), URL);
  await userEvent.type(add.getByLabelText("Clone into"), "/Users/user/code");
  // Named before anything is pressed, as Fleet will name it.
  await expect.element(add.getByRole("group", { name: "Project location" }).getByText("/Users/user/code/storefront")).toBeVisible();
  await add.getByRole("button", { name: /^Clone repository/ }).click();
  return add;
}

const locating = (options: SettingUp = {}) => mount(settingUp(options), SLICES);

// Skipped (#2301): the picker's Add a repository row, which the deck's picker does not carry, which the flight deck replaced. To be ported or dropped before landing.
test.skip("a folder added: the dialog closes, the picker holds it, and Setup is open for it", async () => {
  const onAdded = vi.fn();
  locating({ onAdded });
  const add = await opened();
  await add.getByRole("button", { name: "Choose a folder" }).click();
  await expect.element(add.getByLabelText("Project location")).toHaveValue(CHOSEN);
  await add.getByRole("button", { name: /^Add repository/ }).click();
  expect(onAdded).toHaveBeenCalledWith(CHOSEN);
  await expect.poll(() => dialog().query()).toBeNull();
  await expect.element(page.getByRole("region", { name: "Workspaces" })).toBeVisible();
  // Nobody's Manifest yet, so it reads by its own name.
  await repositoryPicker().click();
  const menu = page.getByRole("menu");
  await expect.element(menu.getByText("Not set up")).toBeVisible();
  await expect.element(menu.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
});

test("nothing served: the dialog opens by itself, nothing reads as a fault, and the first add lands on Setup", async () => {
  const onAdded = vi.fn();
  locating({ repositories: [], onAdded });
  const add = dialog();
  await entered(add);
  // Skipped (#2301): the picker's "Nothing set up yet" reading, which the flight deck replaced: it draws no picker where nothing is served.
  expect(page.getByText(/could not be read/).query()).toBeNull();
  await userEvent.type(add.getByLabelText("Project location"), CHOSEN);
  await add.getByRole("button", { name: /^Add repository/ }).click();
  expect(onAdded).toHaveBeenCalledWith(CHOSEN);
  await expect.element(page.getByRole("region", { name: "Workspaces" })).toBeVisible();
  // The picker is on the Cockpit, not on Setup's page.
  await toPlace("Cockpit");
  await expect.element(repositoryPicker()).toHaveTextContent("scratch");
});

// Skipped (#2301): the picker's Add a repository row, which the deck's picker does not carry, which the flight deck replaced. To be ported or dropped before landing.
test.skip("a clone that finished after its dialog closed says so, and moves nothing until its Setup is asked for", async () => {
  locating({ clone: "late" });
  const add = await cloneFrom();
  await add.getByRole("button", { name: /^Cancel/ }).click();
  await expect.poll(() => dialog().query()).toBeNull();
  await expect.element(page.getByText("storefront is ready to set up")).toBeVisible();
  await expect.element(repositoryPicker()).toHaveTextContent(MANIFEST_NAME);
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  await page.getByRole("button", { name: "Open Setup" }).click();
  await expect.element(page.getByRole("region", { name: "Workspaces" })).toBeVisible();
  await expect.element(repositoryPicker()).toHaveTextContent("storefront");
  await expect.poll(() => page.getByText("storefront is ready to set up").query()).toBeNull();
});

// Skipped (#2301): the picker's Add a repository row, which the deck's picker does not carry, which the flight deck replaced. To be ported or dropped before landing.
test.skip("a late clone is heard in another window on the same main, and neither window's pick moves", async () => {
  const [asked, other] = mountTwo(settingUp({ clone: "late" }), ["The window that asked", "Another window"], SLICES);
  const inAsked = page.elementLocator(asked);
  const inOther = page.elementLocator(other);
  const add = await cloneFrom(inAsked);
  await add.getByRole("button", { name: /^Cancel/ }).click();
  // Present rather than visible: two windows in one test frame are each too narrow to lay the notice out.
  await expect.element(inOther.getByText("storefront is ready to set up")).toBeInTheDocument();
  await expect.element(inAsked.getByText("storefront is ready to set up")).toBeInTheDocument();
  for (const window of [inAsked, inOther]) {
    // Hidden, not absent: each window is narrow, and the picker is behind its menu.
    await expect.element(repositoryPicker(window, { includeHidden: true })).toHaveTextContent(MANIFEST_NAME);
    expect(window.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
  }
  (inOther.getByRole("button", { name: "Open Setup" }).element() as HTMLElement).click();
  await expect.element(inOther.getByRole("region", { name: "Workspaces" })).toBeInTheDocument();
  (inAsked.getByRole("button", { name: "Dismiss" }).element() as HTMLElement).click();
  await expect.poll(() => inAsked.getByText("storefront is ready to set up").query()).toBeNull();
});

test("a parent under a symlink previews the folder Fleet clones into, as main resolves it", async () => {
  locating({ repositories: [] });
  const add = await opened();
  await add.getByRole("button", { name: "Clone from a URL" }).click();
  await userEvent.type(add.getByLabelText("Repository URL"), URL);
  await userEvent.type(add.getByLabelText("Clone into"), "/tmp");
  await expect.element(add.getByRole("group", { name: "Project location" }).getByText("/private/tmp/storefront")).toBeVisible();
  await add.getByRole("button", { name: /^Cancel/ }).click();
});

test("a clone underway says so, and a second press — the button or Enter — sends nothing", async () => {
  const onCloned = vi.fn();
  locating({ repositories: [], clone: "underway", onCloned });
  const add = await cloneFrom();
  await expect.element(add.getByText(/Git is cloning into/)).toBeVisible();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(onCloned).toHaveBeenCalledTimes(1);
  expect(onCloned).toHaveBeenCalledWith(URL, "/Users/user/code");
});

test("git's refusal reads in the dialog in full, and nothing moves behind it", async () => {
  locating({ repositories: [], clone: "refused" });
  const add = await cloneFrom();
  await expect.element(add.getByText("Not cloned")).toBeVisible();
  await expect.element(add.getByText(`git refused the clone: fatal: repository '${URL}' not found.`)).toBeVisible();
  expect(add.getByText("fleet.clone_refused").query()).toBeNull();
  await expect.element(add.getByRole("button", { name: /^Clone repository/ })).toBeEnabled();
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
});

test("a destination already full: Fleet's words, and where to go from them", async () => {
  locating({ repositories: [], clone: "occupied" });
  const add = await cloneFrom();
  await expect.element(add.getByText("Not cloned")).toBeVisible();
  await expect.element(add.getByText("/Users/user/code/storefront already exists and is not empty.")).toBeVisible();
  await expect.element(add.getByText("Choose another folder to clone into.")).toBeVisible();
  expect(add.getByRole("group", { name: "Project location" }).query()).toBeNull();
  expect(add.getByText("fleet.destination_occupied").query()).toBeNull();
});

// Skipped (#2301): the picker's Add a repository row to open the dialog a second time, which the deck's picker does not carry, which the flight deck replaced. To be ported or dropped before landing.
test.skip("Escape closes it with nothing sent, and it opens again empty", async () => {
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
  await expect.element(page.getByRole("menuitem", { name: "storefront" })).toBeInTheDocument();
  // Three repositories would read "api" — one set up and two not — so all three
  // widen. The ladder is one namespace since 28 Sep 2026: a set-up repository
  // used to be exempt because its label was its unique id, and its label is a
  // name now, which can collide like any other.
  await expect.element(page.getByRole("menuitem", { name: "services/api" })).toBeInTheDocument();
  expect(page.getByRole("menuitem", { name: "api", exact: true }).query()).toBeNull();
  await expect.element(page.getByRole("menu").getByText("Not set up")).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  await expect.element(page.getByRole("menuitem", { name: "code/api" })).toBeInTheDocument();
  await expect.element(page.getByRole("menuitem", { name: "old/api" })).toBeInTheDocument();
  expect(page.getByRole("menuitem", { name: "web-app" }).query()).toBeNull();
});
