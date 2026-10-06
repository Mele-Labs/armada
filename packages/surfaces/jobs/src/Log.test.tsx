// Which of the log's rows draw their line as markdown.
//
// **A browser test**, because what is on trial is what the closed row draws:
// the Drone's sentence through `Prose`, every other row's string as it is.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { Log } from "./Log";
import { mount, unmount } from "@armada/screens/src/mounted";
import type { LogRow } from "./story";

afterEach(unmount);

const SAID = "Splitting the reducer. **The public signature** stays put:\n\n- `selectSettings` keeps its name\n- the tests move with it";

/** Fleet's own line, carrying characters markdown would read. */
const NOTE = "Fleet cleared __pending__ for `settle`";

const rows: LogRow[] = [
  { id: "1", at: "09:14:02", actor: "drone", kind: "said", message: SAID, payload: [] },
  { id: "2", at: "09:14:03", actor: "fleet", kind: "note", message: NOTE, payload: [] },
];

test("the Drone's sentence draws its markdown, and Fleet's line stays its characters", async () => {
  mount(<Log rows={rows} emptyNote="Nothing yet" region="log" />);
  await expect.element(page.getByText("The public signature")).toBeVisible();
  expect(page.getByText("The public signature").element().tagName).toBe("STRONG");
  expect(page.getByText("selectSettings").element().tagName).toBe("CODE");
  expect(page.getByText("the tests move with it").element().tagName).toBe("LI");
  await expect.element(page.getByText(NOTE, { exact: true })).toBeVisible();
});
