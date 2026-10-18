// What the layer reads in the page, and what it refuses to carry out of one —
// #1294, `docs/practices/capture-window.md`, *The injected layer*.
//
// **Run against a stub page rather than a DOM**, because main's project has no
// `lib.dom` and this code is not main's to run — it is source that crosses into
// a foreign document. The stub is exactly `PageElement`, which is exactly what
// the layer may touch, so a reading this cannot express is one the layer cannot
// make either.

import { describe, expect, it } from "vitest";

import { SOURCE_ATTRIBUTE } from "../../shared/annotations";
import { ASK_BOUNDS, askSource, readsThePage } from "./layer";

type Built = {
  tag: string;
  attributes?: Record<string, string>;
  text?: string;
  html?: string;
  children?: Built[];
  fiber?: string[];
  /** The element under the pointer, in a page built by `tree`. */
  pick?: true;
};

/** One node of the stub page, with only what `PageElement` declares. */
function node(built: Built, parent: unknown = null): Record<string, unknown> {
  const attributes: Record<string, string> = { ...built.attributes };
  const children: Record<string, unknown>[] = [];
  const it: Record<string, unknown> = {
    tagName: built.tag.toUpperCase(),
    id: attributes["id"] ?? "",
    textContent: built.text ?? "",
    parentElement: parent,
    parentNode: parent,
    get previousElementSibling() {
      const siblings = ((parent as { children?: unknown[] } | null)?.children ?? []) as unknown[];
      return siblings[siblings.indexOf(it) - 1] ?? null;
    },
    children,
    get attributes() {
      return Object.keys(attributes).map((name) => ({ name }));
    },
    getAttribute: (name: string) => attributes[name] ?? null,
    removeAttribute: (name: string) => {
      delete attributes[name];
    },
    closest: (selector: string) => (selector === "svg" ? null : null),
    // Every descendant: the layer re-checks what each selector asks, so the stub need not parse one.
    querySelectorAll: () => descendants(children),
    cloneNode: () => node(built),
    replaceChildren: () => {
      children.length = 0;
      it["textContent"] = "";
      it["emptied"] = true;
    },
    getBoundingClientRect: () => ({ x: 12, y: 34, width: 56, height: 78 }),
    get outerHTML(): string {
      const spelled = Object.entries(attributes)
        .map(([name, value]) => ` ${name}="${value}"`)
        .join("");
      const inside =
        it["emptied"] === true
          ? ""
          : (built.html ?? [...(built.children ?? []).map(() => ""), built.text ?? ""].join(""));
      return `<${built.tag}${spelled}>${inside}</${built.tag}>`;
    },
  };
  // React's own expando, which is the whole reason this runs in the page's world.
  if (built.fiber !== undefined) {
    let fiber: Record<string, unknown> | null = null;
    for (const name of [...built.fiber].reverse()) {
      const named = (): null => null;
      Object.defineProperty(named, "name", { value: name });
      fiber = { type: named, return: fiber };
    }
    it["__reactFiber$abc"] = fiber;
  }
  if (built.pick === true) it["picked"] = true;
  for (const child of built.children ?? []) children.push(node(child, it));
  return it;
}

function descendants(children: Record<string, unknown>[]): Record<string, unknown>[] {
  return children.flatMap((one) => [one, ...descendants(one["children"] as Record<string, unknown>[])]);
}

/** A whole page under `body`, and the ask aimed at the node marked `pick`. */
function tree(body: Built[]): { answer: () => Answer } {
  const root = node({ tag: "body", children: body });
  const all = descendants(root["children"] as Record<string, unknown>[]);
  const picked = all.find((one) => one["picked"] === true) ?? null;
  const globals = globalThis as Record<string, unknown>;
  globals["document"] = {
    elementFromPoint: () => picked,
    querySelector: () => null,
    querySelectorAll: () => all,
    getElementById: (id: string) => all.find((one) => one["id"] === id) ?? null,
  };
  globals["window"] = {
    getComputedStyle: () => ({ getPropertyValue: () => "" }),
    location: { pathname: "/", search: "?walk=burnBySessionModelTool", hash: "" },
    innerWidth: 900,
    innerHeight: 600,
  };
  return { answer: () => readsThePage(10, 20, true, ASK_BOUNDS) as Answer };
}

/** The page the ask runs against, with one element under the pointer. */
function page(under: Built): { answer: (take: boolean) => unknown } {
  const element = node(under);
  const globals = globalThis as Record<string, unknown>;
  globals["document"] = {
    elementFromPoint: () => element,
    querySelector: () => null,
    querySelectorAll: () => [],
    getElementById: () => null,
  };
  globals["window"] = {
    getComputedStyle: () => ({
      getPropertyValue: (property: string) =>
        ({ color: "rgb(1, 2, 3)", cursor: "pointer" })[property] ?? "",
    }),
    location: { pathname: "/checkout", search: "?step=2", hash: "#pay" },
    innerWidth: 900,
    innerHeight: 600,
  };
  return { answer: (take: boolean) => readsThePage(10, 20, take, ASK_BOUNDS) };
}

type Answer = { box: unknown; named: string; capture?: Record<string, unknown> };

describe("what the layer reads", () => {
  it("names the component off React's own expando, which an isolated world cannot see", () => {
    const answer = page({ tag: "button", text: "Pay", fiber: ["PayButton", "Checkout", "App"] }).answer(true) as Answer;
    expect(answer.named).toBe("PayButton");
    expect(answer.capture).toMatchObject({ component: "PayButton", owners: ["Checkout", "App"] });
  });

  it("falls back to the tag on a page React did not draw, and records everything else", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(true) as Answer;
    expect(answer.named).toBe("button");
    expect(answer.capture).toMatchObject({ element: { tag: "button", text: "Pay" }, location: "/checkout?step=2#pay" });
    expect(answer.capture?.["component"]).toBeUndefined();
  });

  it("answers the box and the name alone when it is not asked to take one", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(false) as Answer;
    expect(answer.box).toEqual({ x: 12, y: 34, width: 56, height: 78 });
    expect(answer.capture).toBeUndefined();
  });

  it("keeps only the declared style properties", () => {
    const answer = page({ tag: "button", text: "Pay" }).answer(true) as Answer;
    expect(answer.capture?.["styles"]).toEqual({ color: "rgb(1, 2, 3)" });
  });
});

/** The Usage mock's burn rows, as the owner's note of 17 Oct found them: a tab selected, the Bash list open. */
function usagePage(stamped: boolean): { answer: () => Answer } {
  return tree([
    {
      tag: "main",
      attributes: { "aria-label": "Usage" },
      children: [
        {
          tag: "div",
          attributes: { role: "tablist", "aria-label": "Burn by" },
          children: [
            { tag: "button", attributes: { role: "tab", "aria-selected": "false" }, text: "Session" },
            { tag: "button", attributes: { role: "tab", "aria-selected": "true" }, text: "Bash" },
          ],
        },
        {
          tag: "section",
          attributes: { "aria-label": "Burn", ...(stamped ? { "data-armada-source": "packages/components/src/compositions/UsagePage/UsagePage.tsx:160" } : {}) },
          children: [
            { tag: "h2", text: "Where the burn went" },
            {
              tag: "ul",
              attributes: { "aria-label": "Burn by Bash command" },
              children: [
                { tag: "li", attributes: { "aria-label": "cargo test, 41.0% of the burn" }, text: "cargo test41.0%" },
                {
                  tag: "li",
                  attributes: {
                    "aria-label": "Other commands, 16.1% of the burn",
                    ...(stamped ? { "data-armada-source": "packages/components/src/compositions/UsagePage/UsagePage.tsx:171" } : {}),
                  },
                  text: "Other commands16.1%",
                  fiber: ["ShareRows", "UsagePage", "Boundary"],
                  pick: true,
                },
              ],
            },
          ],
        },
      ],
    },
  ]);
}

describe("what a Session's note reads of the page", () => {
  it("names the places around the row and the heading above it, innermost first", () => {
    const capture = usagePage(false).answer().capture;
    expect(capture?.["nearby"]).toEqual(["Burn by Bash command", "Where the burn went", "Burn", "Usage"]);
  });

  it("reads what the page marks selected, each as `role \"name\"`", () => {
    expect(usagePage(false).answer().capture?.["state"]).toEqual({ selected: ['tab "Bash"'] });
  });

  it("takes the stamp off the nearest stamped element, and none where nothing is stamped", () => {
    expect(usagePage(true).answer().capture?.["stamp"]).toBe("packages/components/src/compositions/UsagePage/UsagePage.tsx:171");
    expect(usagePage(false).answer().capture).not.toHaveProperty("stamp");
  });

  it("still reads the component chain and the label", () => {
    expect(usagePage(false).answer().capture).toMatchObject({
      component: "ShareRows",
      owners: ["UsagePage", "Boundary"],
      element: { tag: "li", label: "Other commands, 16.1% of the burn" },
    });
  });

  it("counts current, pressed, checked, expanded and open, and never `aria-current=\"false\"`", () => {
    const answer = tree([
      { tag: "a", attributes: { "aria-current": "page", href: "/" }, text: "Usage" },
      { tag: "a", attributes: { "aria-current": "false", href: "/x" }, text: "Cockpit" },
      { tag: "button", attributes: { "aria-pressed": "true", "aria-label": "Mono" } },
      { tag: "input", attributes: { type: "checkbox", "aria-checked": "true", "aria-label": "Autoplay" } },
      { tag: "button", attributes: { "aria-expanded": "true" }, text: "More" },
      { tag: "details", attributes: { open: "" }, children: [{ tag: "summary", text: "Why" }] },
      { tag: "dialog", attributes: { open: "", "aria-label": "Settings" }, children: [{ tag: "p", text: "Here", pick: true }] },
    ]).answer();
    expect(answer.capture?.["state"]).toEqual({
      selected: ['link "Usage"', 'button "Mono"', 'checkbox "Autoplay"', 'button "More"', 'group "Why"', 'dialog "Settings"'],
    });
    expect(answer.capture?.["nearby"]).toEqual(["Settings"]);
  });
});

describe("what the markup never carries", () => {
  it("drops a `value` attribute, which is what a person typed", () => {
    const answer = page({ tag: "div", attributes: { value: "4242 4242 4242 4242", class: "row" } }).answer(
      true,
    ) as Answer;
    expect(answer.capture?.["markup"]).not.toContain("4242");
    expect(answer.capture?.["markup"]).toContain('class="row"');
  });

  it("empties a textarea rather than keeping its contents", () => {
    const answer = page({ tag: "textarea", text: "my whole draft", html: "my whole draft" }).answer(true) as Answer;
    expect(answer.capture?.["markup"]).toBe("<textarea></textarea>");
  });

  it("strips a password field to its type and nothing else", () => {
    const answer = page({
      tag: "input",
      attributes: { type: "password", value: "hunter2", name: "password", autocomplete: "off" },
    }).answer(true) as Answer;
    expect(answer.capture?.["markup"]).toBe('<input type="password"></input>');
  });
});

describe("the source of one ask", () => {
  it("is an expression, which is the only channel the page has to main", () => {
    const source = askSource(10, 20, true);
    expect(source.startsWith("(function")).toBe(true);
    expect(source.endsWith(")")).toBe(true);
  });

  it("serialises every argument rather than naming one, so the page cannot change the ask", () => {
    expect(askSource(10, 20, false)).toContain(")(10, 20, false, {");
    expect(askSource(10, 20, false)).toContain('"markup":2000');
  });

  it("leaves nothing in the page: no global is named and no listener is added", () => {
    const source = askSource(1, 2, true);
    expect(source).not.toContain("addEventListener");
    expect(source).not.toContain("window.__");
    expect(source).not.toContain("localStorage");
    expect(source).not.toContain("cookie");
    expect(source).not.toContain("fetch");
  });

  it("reads the stamp the mock build writes, spelled as the build spells it", () => {
    expect(askSource(1, 2, true)).toContain(`"${SOURCE_ATTRIBUTE}"`);
  });
});
