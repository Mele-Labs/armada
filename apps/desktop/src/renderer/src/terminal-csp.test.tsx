// The terminal's colours reach the screen under Bridge's own Content-Security-Policy. xterm writes them as
// `<style>` elements and style attributes, which that policy refuses, and the patch to xterm
// (`patches/@xterm__xterm.patch`) adopts them as constructable stylesheets and sets them through CSSOM.
// The policy is `index.html`'s, put on this page as a meta tag. It lives as long as the page, so this is
// its own file: the page is the test file's.

import { XtermScreen } from "@armada/components";
import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";

// Typed by hand, as `catalogue.ts` does: Vite's client types are not loaded here.
type Globbing = ImportMeta & { glob(pattern: string, options: { eager: true; query: string; import: string }): Record<string, string> };
const [indexHtml = ""] = Object.values((import.meta as Globbing).glob("../index.html", { eager: true, query: "?raw", import: "default" }));

const POLICY = /http-equiv="Content-Security-Policy"\s+content="([^"]+)"/.exec(indexHtml)?.[1];

const colorOfWord = (word: string) => getComputedStyle([...document.querySelectorAll(".xterm-rows span")].find((one) => one.textContent === word)!).color;

test("the policy is in force: an inline <style> is refused", () => {
  expect(POLICY).toContain("default-src 'self'");
  const meta = document.createElement("meta");
  meta.httpEquiv = "Content-Security-Policy";
  meta.content = POLICY!;
  document.head.append(meta);

  const probe = document.createElement("p");
  probe.textContent = "probe";
  document.body.append(probe);
  const style = document.createElement("style");
  style.textContent = "p { color: rgb(1, 2, 3) }";
  document.head.append(style);
  expect(getComputedStyle(probe).color).not.toBe("rgb(1, 2, 3)");
  probe.remove();
  style.remove();
});

test("a screen drawn under it still has the theme's background, its palette colours and 256 and true colours", async () => {
  const host = document.createElement("div");
  host.style.cssText = "width: 640px; height: 240px; display: flex";
  document.body.append(host);
  const out = "\u001b[32mgreen\u001b[39m \u001b[31mred\u001b[39m \u001b[38;5;208morange\u001b[39m \u001b[38;2;10;200;30mlime\u001b[39m\r\n";
  createRoot(host).render(<XtermScreen out={out} run={1} running={false} write={() => undefined} onSize={() => undefined} />);

  await expect.element(page.getByText("lime")).toBeVisible();
  const [green, red, orange, lime] = ["green", "red", "orange", "lime"].map(colorOfWord);
  expect(green).not.toBe(red);
  expect(green).not.toBe("rgb(0, 0, 0)");
  expect(orange).toBe("rgb(255, 135, 0)");
  expect(lime).toBe("rgb(10, 200, 30)");
  expect(getComputedStyle(document.querySelector(".xterm-scrollable-element")!).backgroundColor).toBe("rgb(11, 15, 19)");
});
