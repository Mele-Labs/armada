// Gives the window its theme: reads a `ThemeSource` and keeps the document in step with it.
//
// **The seam Fleet plugs into.** Bridge passes `source` from `main.tsx`; today that is the local
// one with the catalogue and no mods, and the Fleet-backed source that serves `mods/` replaces it
// there. The mock passes one with the Session's mods in it. Everything below and beside this file
// reads the source, never a module.

import { useLayoutEffect, useMemo } from "react";
import type { ReactNode } from "react";
import { ThemeSourceProvider, withoutMods } from "@armada/settings";
import type { ModsSource } from "@armada/settings";

import { createThemeLoader, skipsMods } from "./theme-loader";

type ThemedProps = { source: ModsSource; noMods?: boolean; children?: ReactNode };

export function Themed({ source, noMods = skipsMods(), children }: ThemedProps) {
  const shown = useMemo(() => (noMods ? withoutMods(source) : source), [source, noMods]);

  // Layout, not passive: a mod's or Dark's theme is on the document before the first paint. A
  // catalogue theme arrives a moment after, as its file is read.
  useLayoutEffect(() => {
    const loader = createThemeLoader(document);
    // A later choice supersedes an earlier one still being read.
    let latest = 0;
    const sync = () => {
      const mine = (latest += 1);
      const finish = (id: string, css: string | null) => {
        if (mine !== latest) return;
        const applied = loader.apply(id, css);
        // The loader fell back, so the source and the picker say Dark too.
        if (applied !== id) shown.setActive(applied);
      };
      try {
        const { active, mods, catalogue } = shown.get();
        const mod = mods.find((one) => one.name === active && one.enabled);
        const shipped = catalogue.find((one) => one.id === active);
        if (mod !== undefined) finish(active, mod.css);
        else if (shipped !== undefined) shipped.load().then((css) => finish(active, css), (why) => (console.warn("theme not read", why), finish(active, null)));
        else finish(active, null);
      } catch (why) {
        console.warn("theme source failed, using Dark", why);
        loader.apply("dark", null);
      }
    };
    sync();
    const off = shown.subscribe(sync);
    return () => {
      latest += 1;
      off();
      loader.clear();
    };
  }, [shown]);

  return <ThemeSourceProvider value={shown}>{children}</ThemeSourceProvider>;
}
