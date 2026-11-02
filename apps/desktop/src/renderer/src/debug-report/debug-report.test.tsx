// The Copy debug report sheet: what the reader gives is what shows and what Copy writes, Esc gives focus back to the
// control that opened it, and a reader that fails leaves one line and no Copy. Browser: a rendering is read.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { mount, unmount } from "@armada/screens/src/mounted";

import { DebugReportFrom, useDebugReport, type DebugRead } from "./debug-report";

const writeText = vi.fn(async (_text: string) => {});

beforeEach(() => {
  writeText.mockClear();
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
});
afterEach(unmount);

function Opener() {
  const report = useDebugReport();
  return (
    <button type="button" onClick={(event) => report?.open(event.currentTarget)}>
      Open report
    </button>
  );
}

function Offered() {
  return <p>{useDebugReport() === null ? "no report" : "report"}</p>;
}

const sheet = (read: DebugRead | undefined) => (
  <DebugReportFrom read={read}>
    <Opener />
  </DebugReportFrom>
);

const TEXT = "Armada debug report\n\nInstall\n  armada   0.14.2\n\nbridge protocol 5.2  fleet protocol 5.2  taken 2026-11-02T09:31:12Z";

const shown = () => page.getByRole("dialog", { name: "Debug report" }).element().querySelector("pre")?.textContent;

test("the reader's text is what shows, and exactly what Copy writes", async () => {
  mount(sheet(async () => TEXT));
  await userEvent.click(page.getByRole("button", { name: "Open report" }));
  await expect.poll(shown).toBe(TEXT);
  await userEvent.click(page.getByRole("dialog", { name: "Debug report" }).getByRole("button", { name: "Copy", exact: true }));
  await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith(TEXT));
  expect(writeText).toHaveBeenCalledTimes(1);
});

test("a read that has not answered shows the placeholder and offers no Copy", async () => {
  mount(sheet(() => new Promise<string>(() => undefined)));
  await userEvent.click(page.getByRole("button", { name: "Open report" }));
  const dialog = page.getByRole("dialog", { name: "Debug report" });
  await expect.element(dialog.getByLabelText("Reading the report")).toBeVisible();
  await expect.element(dialog.getByRole("button", { name: "Copy", exact: true })).not.toBeInTheDocument();
});

test("Esc closes the sheet and focus is back on the control that opened it", async () => {
  mount(sheet(async () => TEXT));
  const opener = page.getByRole("button", { name: "Open report" });
  await userEvent.click(opener);
  await expect.element(page.getByRole("dialog", { name: "Debug report" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("dialog", { name: "Debug report" })).not.toBeInTheDocument();
  await expect.element(opener).toHaveFocus();
});

test("a reader that rejects shows one line saying why, and no Copy", async () => {
  mount(sheet(async () => Promise.reject(new Error("no armada binary was found"))));
  await userEvent.click(page.getByRole("button", { name: "Open report" }));
  const dialog = page.getByRole("dialog", { name: "Debug report" });
  await expect.element(dialog.getByRole("alert")).toHaveTextContent("The report could not be read: no armada binary was found");
  await expect.element(dialog.getByRole("button", { name: "Copy", exact: true })).not.toBeInTheDocument();
  expect(shown()).toBeUndefined();
});

test("each opening reads again", async () => {
  const read = vi.fn<DebugRead>().mockResolvedValueOnce("first").mockResolvedValueOnce("second");
  mount(sheet(read));
  const opener = page.getByRole("button", { name: "Open report" });
  await userEvent.click(opener);
  await expect.poll(shown).toBe("first");
  await userEvent.keyboard("{Escape}");
  await userEvent.click(opener);
  await expect.poll(shown).toBe("second");
  expect(read).toHaveBeenCalledTimes(2);
});

test("a window with no reader offers nothing", async () => {
  mount(
    <DebugReportFrom read={undefined}>
      <Offered />
    </DebugReportFrom>,
  );
  await expect.element(page.getByText("no report")).toBeVisible();
});
