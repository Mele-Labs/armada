// Bridge blurs nothing behind its panels. Any frame redraws the window, and a blur makes that frame several times dearer: one row's fade at the
// display's rate read 26.5% GPU on the mock with the blur and 3.9% without. Browser: the real stylesheets, read as the browser parsed them.

import { expect, test } from "vitest";

import "./styles/index.css";

test("no rule in Bridge's stylesheets blurs what is behind it", () => {
  const blurring: string[] = [];
  const walk = (rules: CSSRuleList): void => {
    for (const rule of Array.from(rules)) {
      if (
        rule instanceof CSSStyleRule &&
        ![
          "",
          "none",
        ].includes(rule.style.backdropFilter)
      )
        blurring.push(rule.selectorText);
      if ("cssRules" in rule) walk((rule as CSSGroupingRule).cssRules);
    }
  };
  for (const sheet of Array.from(document.styleSheets)) walk(sheet.cssRules);
  expect(blurring).toEqual([]);
});
