// The lens's card under reduced motion, as every browser suite runs: a hunk with lines new since the last read shows whole at once.

import { createRoot } from "react-dom/client";
import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { lensOf, newestOf } from "../cockpit/deck/lens";
import { FieldLensCard } from "./FieldLensCard";

const diff = (n: number) => `diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n@@ -1,1 +1,${1 + n} @@\n top\n${Array.from({ length: n }, (_, k) => `+line ${k}\n`).join("")}`;

test("under reduced motion the new lines of the newest hunk show whole at once, and nothing types", async () => {
  const was = lensOf({ ok: true, stamp: "1", diff: diff(1) });
  const now = lensOf({ ok: true, stamp: "2", diff: diff(3) });
  const newest = newestOf(was, now)!;
  expect(newest.read).toBe(2);
  const typing = vi.fn();
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  root.render(<FieldLensCard card={{ path: "src/a.ts", diff: now, newest: { ...newest, n: 1 }, tabs: [{ path: "src/b.ts", diff: undefined }] }} large={false} onTyping={typing} />);
  const card = page.getByRole("group", { name: "Lens: a.ts" });
  await expect.element(card).toHaveTextContent("+3 −0");
  await expect.element(card).toHaveTextContent("line 2");
  await expect.element(card).toHaveTextContent("b.ts");
  expect(typing).not.toHaveBeenCalledWith(true);
  root.unmount();
  host.remove();
});
