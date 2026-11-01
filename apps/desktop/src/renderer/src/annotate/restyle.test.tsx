// What a note's changes do to the page while its card is up, and that none is left behind when it goes.
// A `.tsx` because it needs a real document, which the desktop project's node environment has none of;
// it mounts no React.

import { afterEach, describe, expect, it } from "vitest";

import type { StyleChange } from "../../../shared/annotations";
import { asHex, computed, putOn, RESTYLED, takeOff, tokenValue, wordsOf } from "./restyle";

const held: Element[] = [];

/** An element on the page, with an optional child, so a computed style is one the page draws. */
function onPage(html: string, style = ""): HTMLElement {
  const host = document.createElement("div");
  host.innerHTML = html;
  const element = host.firstElementChild as HTMLElement;
  if (style !== "") element.style.cssText = style;
  document.body.append(host);
  held.push(host);
  return element;
}

afterEach(() => {
  for (const one of held.splice(0)) one.remove();
  document.querySelectorAll("[data-armada-restyle]").forEach((one) => one.remove());
  document.documentElement.removeAttribute("style");
});

const size = (after: string, token?: string): StyleChange => ({ kind: "size", property: "font-size", before: "13px", after, ...(token === undefined ? {} : { token }) });
const colour = (after: string, token?: string): StyleChange => ({ kind: "colour", property: "color", before: "#FFFFFF", after, ...(token === undefined ? {} : { token }) });
const fill = (after: string, token?: string): StyleChange => ({ kind: "fill", property: "background-color", before: "transparent", after, ...(token === undefined ? {} : { token }) });
const padding = (after: string, token?: string): StyleChange => ({ kind: "spacing", property: "padding", before: "0px", after, ...(token === undefined ? {} : { token }) });
const words = (before: string, after: string): StyleChange => ({ kind: "text", property: "text", before, after });

describe("asHex", () => {
  it("writes a computed colour as the tokens are, in capitals", () => {
    expect(asHex("rgb(74, 158, 219)")).toBe("#4A9EDB");
    expect(asHex("rgb(0, 0, 0)")).toBe("#000000");
  });

  it("keeps a partial alpha as two digits more, and says transparent where none is left", () => {
    expect(asHex("rgba(74, 158, 219, 0.5)")).toBe("#4A9EDB80");
    expect(asHex("rgb(74 158 219 / 50%)")).toBe("#4A9EDB80");
    expect(asHex("rgba(0, 0, 0, 0)")).toBe("transparent");
    expect(asHex("rgba(10, 20, 30, 1)")).toBe("#0A141E");
  });

  it("returns anything else as it came", () => {
    for (const value of ["", "transparent", "red", "var(--accent)"]) expect(asHex(value)).toBe(value);
  });
});

describe("what the page draws", () => {
  it("reads a property as computed, and a token where the element sits", () => {
    const inner = onPage('<div style="--accent: #123456; font-size: 17px">x</div>');
    expect(computed(inner, "font-size")).toBe("17px");
    expect(tokenValue(inner, "--accent")).toBe("#123456");
    expect(tokenValue(inner, "--nothing-like-it")).toBe("");
  });

  it("finds the first words in an element, skipping the blanks between its tags", () => {
    const element = onPage("<div>\n  <span></span> <b> hello </b><i>world</i></div>");
    expect(wordsOf(element)?.nodeValue).toBe(" hello ");
    expect(wordsOf(onPage("<div> <span> </span></div>"))).toBeNull();
  });
});

describe("a note's changes on its element", () => {
  it("puts Fill and Spacing on the element, as the token so a theme moves them, and gives them back", () => {
    document.documentElement.style.setProperty("--bg-raised", "rgb(1, 2, 3)");
    document.documentElement.style.setProperty("--space-4", "16px");
    const element = onPage("<div>words</div>", "padding: 2px");
    putOn(element, [fill("#010203", "--bg-raised"), padding("16px", "--space-4")], "n1");
    expect(element.style.getPropertyValue("background-color")).toBe("var(--bg-raised)");
    expect(computed(element, "background-color")).toBe("rgb(1, 2, 3)");
    expect(computed(element, "padding-top")).toBe("16px");
    // Followed by the theme, not frozen at what it was when picked.
    document.documentElement.style.setProperty("--bg-raised", "rgb(9, 9, 9)");
    expect(computed(element, "background-color")).toBe("rgb(9, 9, 9)");

    takeOff(element);
    expect(element.style.cssText).toBe("padding: 2px;");
  });

  it("writes a value as it is where no token was picked", () => {
    const element = onPage("<div>words</div>");
    putOn(element, [fill("#112233")], "n1");
    expect(computed(element, "background-color")).toBe("rgb(17, 34, 51)");
    takeOff(element);
    expect(element.style.getPropertyValue("background-color")).toBe("");
  });

  it("beats the element's own styling, and puts back the value and the priority it had", () => {
    const element = onPage("<div>words</div>");
    element.style.setProperty("padding", "3px", "important");
    putOn(element, [padding("20px")], "n1");
    expect(computed(element, "padding-left")).toBe("20px");
    takeOff(element);
    expect(element.style.getPropertyValue("padding")).toBe("3px");
    expect(element.style.getPropertyPriority("padding")).toBe("important");
  });

  it("makes Size and Colour reach everything inside the element, over what the inside sets itself", () => {
    document.documentElement.style.setProperty("--text-xl", "22px");
    document.documentElement.style.setProperty("--fg-default", "rgb(10, 11, 12)");
    const element = onPage('<div><span style="font-size: 10px; color: rgb(5, 5, 5)">inner</span><b>bold</b></div>');
    const inner = element.querySelector("span")!;
    putOn(element, [size("22px", "--text-xl"), colour("#0A0B0C", "--fg-default")], "n1");
    expect(computed(element, "font-size")).toBe("22px");
    expect(computed(inner, "font-size")).toBe("22px");
    expect(computed(inner, "color")).toBe("rgb(10, 11, 12)");
    expect(computed(element.querySelector("b")!, "font-size")).toBe("22px");
    // One rule of the note's own does it, and its marker is on the element only while it is up.
    expect(element.getAttribute(RESTYLED)).toBe("n1");
    expect(document.querySelectorAll("style[data-armada-restyle]")).toHaveLength(1);

    takeOff(element);
    expect(computed(inner, "font-size")).toBe("10px");
    expect(computed(inner, "color")).toBe("rgb(5, 5, 5)");
    expect(element.hasAttribute(RESTYLED)).toBe(false);
    expect(document.querySelector("style[data-armada-restyle]")?.textContent).toBe("");
  });

  it("follows the theme for a Size picked as a token", () => {
    document.documentElement.style.setProperty("--text-lg", "20px");
    const element = onPage("<div>words</div>");
    putOn(element, [size("20px", "--text-lg")], "n1");
    expect(computed(element, "font-size")).toBe("20px");
    document.documentElement.style.setProperty("--text-lg", "30px");
    expect(computed(element, "font-size")).toBe("30px");
  });

  it("changes the words, and puts them back", () => {
    const element = onPage("<div><b>Fix the flaky test</b> now</div>");
    putOn(element, [words("Fix the flaky test", "Fix the flaky tests")], "n1");
    expect(element.textContent).toBe("Fix the flaky tests now");
    takeOff(element);
    expect(element.textContent).toBe("Fix the flaky test now");
  });

  it("leaves an element with no words as it is where Text is asked", () => {
    const element = onPage("<div><span></span></div>");
    putOn(element, [words("", "hello")], "n1");
    expect(element.textContent).toBe("");
  });

  it("takes what was put on before off first, so a changed list replaces rather than adds", () => {
    const element = onPage("<div>words</div>");
    putOn(element, [fill("#112233"), size("22px")], "n1");
    putOn(element, [padding("12px")], "n1");
    expect(element.style.getPropertyValue("background-color")).toBe("");
    expect(element.hasAttribute(RESTYLED)).toBe(false);
    expect(computed(element, "padding-top")).toBe("12px");
    putOn(element, [], "n1");
    expect(element.style.cssText).toBe("");
  });

  it("keeps two notes' rules apart, and taking one off leaves the other's", () => {
    const one = onPage("<div>one</div>");
    const two = onPage("<div>two</div>");
    putOn(one, [size("22px")], "n1");
    putOn(two, [size("24px")], "n2");
    expect(computed(one, "font-size")).toBe("22px");
    expect(computed(two, "font-size")).toBe("24px");
    takeOff(one);
    expect(computed(one, "font-size")).not.toBe("22px");
    expect(computed(two, "font-size")).toBe("24px");
  });

  it("takes off nothing from an element nothing was put on", () => {
    const element = onPage("<div>words</div>", "padding: 2px");
    takeOff(element);
    expect(element.style.cssText).toBe("padding: 2px;");
  });
});
