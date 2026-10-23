// Puts the theme tweaks on the document. A tweak is a shared colour token set over whichever theme is chosen, so it sits in sheets of
// its own and is not the theme's: choosing another theme replaces the theme's sheet and leaves these. Constructable, as the theme's
// is, because the window's CSP refuses an inline `<style>`.
//
// **Two things lay a tweak down**: a tweak mod that is on, from `fleet-tweaks.ts`, in mod name order; and the tweak the Teach panel is
// showing back, before it is kept, so the person sees every place the colour is drawn.
//
// **`:root:root` on every selector is for the cascade**: a theme's own block is `:root[data-theme="…"]`, which outranks a bare `:root`
// whichever sheet was adopted last. Two more classes on the selector put a tweak above it, so a theme chosen after the tweak cannot draw
// over it. The declarations are the ones Fleet checked, set on the parsed rules and not rewritten as text.

import { createContext, useContext, useLayoutEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import { NO_TWEAKS } from "./fleet-tweaks";
import type { TweakSheet, TweakSource } from "./fleet-tweaks";
import { usePreviewTweak } from "./teach";

const TweaksContext = createContext<TweakSource>(NO_TWEAKS);

/** The tweaks this window lays over its theme. A window without a provider lays none. */
export const TweakSourceProvider = ({ source, children }: { source: TweakSource; children?: ReactNode }) => <TweaksContext.Provider value={source}>{children}</TweaksContext.Provider>;

const BOOST = ":root:root";

/** A stylesheet of a tweak mod's checked CSS, every selector raised above any theme's. */
export function tweakSheet(css: string): CSSStyleSheet {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(css);
  for (const rule of Array.from(sheet.cssRules)) {
    if (rule instanceof CSSStyleRule) rule.selectorText = rule.selectorText.split(",").map((part) => `${part.trim()}${BOOST}`).join(", ");
  }
  return sheet;
}

const TOKEN = /^-{2}[a-z0-9-]+$/;

/** The one token the Teach panel shows back, as a sheet. Set through the rule, so a value cannot end its declaration. */
function previewSheet(token: string, now: string): CSSStyleSheet | undefined {
  if (!TOKEN.test(token)) return undefined;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`:root${BOOST} {}`);
  (sheet.cssRules[0] as CSSStyleRule).style.setProperty(token, now);
  return sheet;
}

export function TweaksApplied(): null {
  const source = useContext(TweaksContext);
  const mods: readonly TweakSheet[] = useSyncExternalStore(source.subscribe, source.get, source.get);
  const preview = usePreviewTweak();
  const key = JSON.stringify([mods, preview]);
  useLayoutEffect(() => {
    const sheets = [...mods.map((one) => tweakSheet(one.css)), ...(preview === undefined ? [] : [previewSheet(preview.token, preview.now)])].filter((one): one is CSSStyleSheet => one !== undefined);
    if (sheets.length === 0) return;
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, ...sheets];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((one) => !sheets.includes(one));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `key` holds what `mods` and `preview` hold.
  }, [key]);
  return null;
}
