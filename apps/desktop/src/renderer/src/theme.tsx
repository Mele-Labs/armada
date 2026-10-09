// Gives the window its theme: reads a `ThemeSource` and keeps the document in step with it.
//
// **The seam Fleet plugs into.** Bridge passes `source` from `main.tsx`; today that is the local,
// mod-less one, and the Fleet-backed source that serves `mods/` replaces it there. The mock passes
// one with mods in it. Everything below and beside this file reads the source, never a module.

import { useLayoutEffect, useMemo } from "react";
import type { ReactNode } from "react";
import { ThemeSourceProvider, withoutMods } from "@armada/settings";
import type { ModsSource } from "@armada/settings";

import { createThemeLoader, skipsMods } from "./theme-loader";

type ThemedProps = { source: ModsSource; noMods?: boolean; children: ReactNode };

export function Themed({ source, noMods = skipsMods(), children }: ThemedProps) {
  const shown = useMemo(() => (noMods ? withoutMods(source) : source), [source, noMods]);

  // Layout, not passive: the theme is on the document before the first paint, not a frame after.
  useLayoutEffect(() => {
    const loader = createThemeLoader(document);
    const sync = () => {
      try {
        const asked = shown.get();
        const applied = loader.apply(asked);
        // The loader fell back, so the source and the picker say Dark too.
        if (applied !== asked.active) shown.setActive(applied);
      } catch (why) {
        console.warn("theme source failed, using Dark", why);
        loader.apply({ mods: [], active: "dark" });
      }
    };
    sync();
    const off = shown.subscribe(sync);
    return () => {
      off();
      loader.clear();
    };
  }, [shown]);

  return <ThemeSourceProvider value={shown}>{children}</ThemeSourceProvider>;
}
