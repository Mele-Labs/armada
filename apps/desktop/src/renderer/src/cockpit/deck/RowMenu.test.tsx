// A Session row's context menu carries Report, so the row can be reported like anything else.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../../styles/index.css";
import { RowMenu } from "./RowMenu";

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
});

test("the menu lists Report, and choosing it asks to report the row", async () => {
  const row = document.createElement("button");
  row.setAttribute("data-noted", "");
  document.body.append(row);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  root.render(<RowMenu at={{ id: "s-1", root: "/repo", x: 20, y: 20, row }} onClose={() => undefined} onAutopilot={() => undefined} />);

  const asked = vi.fn();
  window.addEventListener("armada:report", asked);
  await page.getByRole("menuitem", { name: "Report" }).click();
  window.removeEventListener("armada:report", asked);

  expect(asked).toHaveBeenCalledTimes(1);
  expect((asked.mock.calls[0]?.[0] as CustomEvent<Element>).detail).toBe(row);
});
