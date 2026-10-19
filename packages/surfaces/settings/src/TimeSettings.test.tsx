// Settings → Time: each of the three switches is kept as it is pressed, and the sample beside them is
// the current moment in the format they make. `TimeSettings` alone, with a saver standing where Bridge
// hands in Fleet's: the claim is the switch and the sample, not the Settings screen around them.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { DEFAULT_TIME_FORMAT, setTimeFormat, setTimeFormatSaver } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

import { TimeSettings } from "./TimeSettings";

const saved = vi.fn<(text: string) => void>();

beforeEach(() => {
  saved.mockClear();
  setTimeFormat(DEFAULT_TIME_FORMAT);
  setTimeFormatSaver(saved);
});
afterEach(() => {
  unmount();
  setTimeFormatSaver(null);
  setTimeFormat(DEFAULT_TIME_FORMAT);
});

/** What was last sent to be kept, read back from its text. */
const lastSaved = (): unknown => JSON.parse(saved.mock.lastCall![0]);

/** The sample is the one stamp on the page, and its name is the reading it draws. */
const sample = () => page.getByRole("button");

test("the switches start as the shipped format and the sample is its clock", async () => {
  mount(<TimeSettings />);

  await expect.element(page.getByRole("switch", { name: "24-hour clock" })).toBeChecked();
  await expect.element(page.getByRole("switch", { name: "Seconds" })).toBeChecked();
  await expect.element(page.getByRole("switch", { name: "Date" })).not.toBeChecked();
  await expect.element(sample()).toHaveAccessibleName(/^\d{2}:\d{2}:\d{2}$/);
  expect(saved).not.toHaveBeenCalled();
});

test("turning 24-hour clock off keeps a 12-hour one and draws the sample with AM or PM", async () => {
  mount(<TimeSettings />);

  await page.getByText("24-hour clock").click();

  expect(lastSaved()).toEqual({ hour12: true, seconds: true, date: false });
  await expect.element(page.getByRole("switch", { name: "24-hour clock" })).not.toBeChecked();
  await expect.element(sample()).toHaveAccessibleName(/^\d{1,2}:\d{2}:\d{2}\s[AP]M$/);
});

test("turning Seconds off keeps it, and the sample loses its seconds", async () => {
  mount(<TimeSettings />);

  await page.getByText("Seconds").click();

  expect(lastSaved()).toEqual({ hour12: false, seconds: false, date: false });
  await expect.element(page.getByRole("switch", { name: "Seconds" })).not.toBeChecked();
  await expect.element(sample()).toHaveAccessibleName(/^\d{2}:\d{2}$/);
});

test("turning Date on keeps it, and the sample gains the day", async () => {
  mount(<TimeSettings />);

  await page.getByText("Date", { exact: true }).click();

  expect(lastSaved()).toEqual({ hour12: false, seconds: true, date: true });
  await expect.element(page.getByRole("switch", { name: "Date" })).toBeChecked();
  await expect.element(sample()).toHaveAccessibleName(/^[A-Z][a-z]{2} \d{1,2} \d{2}:\d{2}:\d{2}$/);
});

test("each press keeps the switches before it, and a switch turned back off is kept off", async () => {
  mount(<TimeSettings />);

  await page.getByText("Date", { exact: true }).click();
  await page.getByText("Seconds").click();
  expect(lastSaved()).toEqual({ hour12: false, seconds: false, date: true });
  await expect.element(sample()).toHaveAccessibleName(/^[A-Z][a-z]{2} \d{1,2} \d{2}:\d{2}$/);

  await page.getByText("Date", { exact: true }).click();
  expect(lastSaved()).toEqual({ hour12: false, seconds: false, date: false });
  await expect.element(sample()).toHaveAccessibleName(/^\d{2}:\d{2}$/);
  expect(saved).toHaveBeenCalledTimes(3);
});
