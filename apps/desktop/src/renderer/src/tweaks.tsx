// Puts the theme tweaks on the document. A tweak is a shared colour token set over whichever theme is chosen, so it sits in a sheet of
// its own and is not the theme's: choosing another theme replaces the theme's sheet and leaves this one. Constructable, as the theme's
// is, because the window's CSP refuses an inline `<style>`.
//
// **`:root:root:root` is for the cascade**: a theme's own block is `:root[data-theme="…"]` at most, and three classes outrank it
// whichever sheet was adopted last, so a theme chosen after the tweak cannot draw over it.

import { useLayoutEffect } from "react";

import { useTweaks } from "./teach";

export function TweaksApplied(): null {
  const css = useTweaks()
    .map((one) => `:root:root:root { ${one.token}: ${one.now}; }`)
    .join("\n");
  useLayoutEffect(() => {
    if (css === "") return;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(css);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((one) => one !== sheet);
    };
  }, [css]);
  return null;
}
