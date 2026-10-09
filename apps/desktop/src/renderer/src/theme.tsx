// Gives the window its theme: reads a `ThemeSource` and keeps the document in step with it.
//
// **The seam Fleet plugs into.** Bridge passes `source` from `main.tsx`, the one `fleet-themes.ts`
// builds over Fleet's mod list and the saved preference. The mock passes one with the Session's
// mods in it. Everything below and beside this file reads the source, never a module.
//
// A mod's CSS is asked for when its theme is drawn (`ThemeMod.load`), so the sheet adopted is the
// text Fleet just checked, and a mod that fails the check is Dark.

import { useLayoutEffect, useMemo } from "react";
import type { ReactNode } from "react";
import { ThemeSourceProvider, offered, withoutMods } from "@armada/settings";
import type { ModsSource } from "@armada/settings";

import { createThemeLoader, skipsMods } from "./theme-loader";

type ThemedProps = { source: ModsSource; noMods?: boolean; children?: ReactNode };

export function Themed({ source, noMods = skipsMods(), children }: ThemedProps) {
  const shown = useMemo(() => (noMods ? withoutMods(source) : source), [source, noMods]);

  // Layout, not passive: Dark's or Light's theme is on the document before the first paint. A
  // catalogue theme or a mod's arrives a moment after, as its file is read or checked.
  useLayoutEffect(() => {
    const loader = createThemeLoader(document);
    // A later choice supersedes an earlier one still being read.
    let latest = 0;
    const sync = () => {
      const mine = (latest += 1);
      const finish = (id: string, css: string | null) => {
        if (mine !== latest) return;
        const applied = loader.apply(id, css);
        // The loader fell back, so the source and the picker say Dark too. A source that keeps the
        // choice keeps it: the mod may pass its next check.
        if (applied === id) return;
        if (shown.fellBack !== undefined) shown.fellBack(id);
        else shown.setActive(applied);
      };
      try {
        const { active, mods, catalogue } = shown.get();
        const loads = (mods.find((one) => one.name === active && offered(one)) ?? catalogue.find((one) => one.id === active))?.load;
        if (loads !== undefined) loads().then((css) => finish(active, css), (why) => (console.warn("theme not read", why), finish(active, null)));
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
