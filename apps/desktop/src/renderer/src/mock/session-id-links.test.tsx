// A Session id in a thread links to that Session, written in backticks or plain, and an ended Session's
// too. Code that is more than the id stays code. Through `App`.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { s208SessionIdLinks } from "./scenarios/session-id-links";

unmountAfterEach();

const thread = () => page.getByRole("region", { name: "Thread" });

test("a Session address in backticks, and one in plain text, each open that Session", async () => {
  mount(s208SessionIdLinks);
  await onScreen();
  await toSession(/Hand-off thread/);

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
  await toSession(/Hand-off thread/);
  await userEvent.click(thread().getByRole("link", { name: "s-e165b4df", exact: true }));
  await expect.element(thread().getByText("Picking this up from")).toBeVisible();
});

async function forge() {
  const app = mount(s208SessionIdLinks);
  await onScreen();
  await toSession(/Cart rounding hand-off/);
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

test("the Session's own pull request opens in its panel", async () => {
  await forge();
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

test("a web address in code is a link, in a span and in a block, and a #N in code is not", async () => {
  const { link } = await forge();
  // `#418` in a span is code: only the sentence above it links it.
  await expect.element(thread().getByRole("code").filter({ hasText: "#418" })).toBeVisible();
  expect(thread().getByRole("link", { name: "#418", exact: true }).elements()).toHaveLength(1);
  await userEvent.click(thread().getByRole("link", { name: "https://example.com/health" }));
  await userEvent.click(thread().getByRole("link", { name: "https://example.com/ready" }));
  await expect.poll(() => link.mock.calls.map((one) => one[0])).toEqual(["https://example.com/health", "https://example.com/ready"]);
  // The comment in the block names #412 too; only the sentence above it is a link.
  expect(thread().getByRole("link", { name: "#412", exact: true }).elements()).toHaveLength(1);
});

const WALK = "http://localhost:47331/?walk=session-id-links";

test("a walk link opens in the Session's walk window, in text and in code, and another localhost address opens as a web link", async () => {
  const app = mount(s208SessionIdLinks);
  await onScreen();
  await toSession(/Walk links/);
  const browser = vi.spyOn(app.api, "openLink").mockResolvedValue({ ok: true });
  const window = page.getByRole("dialog", { name: "Bridge's window on Walk links" });

  // Plain text, a markdown link to the same walk on 127.0.0.1, inline code, and a fenced block.
  const links = [
    [thread().getByRole("link", { name: WALK }).nth(0)],
    [thread().getByRole("link", { name: "the same" })],
    [thread().getByRole("link", { name: WALK }).nth(1)],
    [thread().getByRole("link", { name: WALK }).nth(2)],
  ] as const;
  for (const [link] of links) {
    await userEvent.click(link);
    await expect.element(window).toBeVisible();
    await userEvent.click(page.getByRole("button", { name: "Close window" }));
    await expect.element(window).not.toBeInTheDocument();
  }
  expect(browser).not.toHaveBeenCalled();

  await userEvent.click(thread().getByRole("link", { name: "http://localhost:47331/docs" }));
  await expect.poll(() => browser.mock.calls.length).toBe(1);
  expect(browser).toHaveBeenLastCalledWith("http://localhost:47331/docs");
  await expect.element(window).not.toBeInTheDocument();
});
