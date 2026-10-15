// A Session id in a thread links to that Session, written in backticks or plain, and an ended Session's
// too. Code that is more than the id stays code. Through `App`; the walk `session-id-links` draws it.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { s208SessionIdLinks } from "./scenarios/session-id-links";

unmountAfterEach();

const thread = () => page.getByRole("region", { name: "Thread" });

test("a Session address in backticks, and one in plain text, each open that Session", async () => {
  mount(s208SessionIdLinks);
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Hand-off thread/ }));

  // Code that is more than an address stays code.
  await expect.element(thread().getByText("s-e165b4df --fix")).toBeVisible();
  await expect.element(thread().getByRole("link", { name: "s-e165b4df --fix" })).not.toBeInTheDocument();

  // The ended Session still has its page, so its address is a link.
  await userEvent.click(thread().getByRole("link", { name: "s-b7d20c9e" }));
  await expect.element(thread().getByText("Nothing more to add.")).toBeVisible();
});

test("an address in backticks links and opens its Session", async () => {
  mount(s208SessionIdLinks);
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Hand-off thread/ }));
  await userEvent.click(thread().getByRole("link", { name: "s-e165b4df", exact: true }));
  await expect.element(thread().getByText("Picking this up from")).toBeVisible();
});

async function forge() {
  const app = mount(s208SessionIdLinks);
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Cart rounding hand-off/ }));
  const link = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  return { link };
}

test("a merged pull request in another repository's Session links, and opens on the forge", async () => {
  const { link } = await forge();
  // Looked up in the Session's own repository, which is not Helm's: #2207 is only armada's.
  await expect.element(thread().getByRole("link", { name: "#401", exact: true })).toBeVisible();
  await expect.element(thread().getByRole("link", { name: "#2207" })).not.toBeInTheDocument();
  await userEvent.click(thread().getByRole("link", { name: "#401", exact: true }));
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://forge.invalid/pull/401");
});

test("an issue opens on the forge, and the Session's own pull request in its panel", async () => {
  const { link } = await forge();
  await userEvent.click(thread().getByRole("link", { name: "#412", exact: true }));
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://forge.invalid/issues/412");
  await userEvent.click(thread().getByRole("link", { name: "#2301", exact: true }));
  await expect.element(page.getByRole("dialog", { name: "Pull request #2301" })).toBeVisible();
});

test("a pull request's address draws as the reference, with its title on hover", async () => {
  const { link } = await forge();
  const address = thread().getByRole("link", { name: "https://forge.invalid/pull/401", exact: true });
  await expect.element(address).toBeVisible();
  // A reference is described by its tooltip; a bare web link carries its address as a title.
  await expect.element(address).toHaveAttribute("aria-describedby");
  await expect.element(address).not.toHaveAttribute("title");
  await userEvent.click(address);
  await expect.poll(() => link.mock.calls.length).toBe(1);
  expect(link).toHaveBeenLastCalledWith("https://forge.invalid/pull/401");
});

test("an address in code is a link, in a span and in a block, and a #N in a block is not", async () => {
  const { link } = await forge();
  // `#418` in code links as it does in the sentence above it: two links.
  await expect.element(thread().getByRole("link", { name: "#418", exact: true }).nth(1)).toBeVisible();
  await userEvent.click(thread().getByRole("link", { name: "https://example.com/health" }));
  await userEvent.click(thread().getByRole("link", { name: "https://example.com/ready" }));
  await expect.poll(() => link.mock.calls.map((one) => one[0])).toEqual(["https://example.com/health", "https://example.com/ready"]);
  // The comment in the block names #412 too; only the sentence above it is a link.
  expect(thread().getByRole("link", { name: "#412", exact: true }).elements()).toHaveLength(1);
});
