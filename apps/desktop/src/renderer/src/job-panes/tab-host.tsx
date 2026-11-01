// A built Custom tab's body: the tab mod's module, run in place. It is loaded again each time Fleet's list says the mod's files changed, so what the Mod Job writes replaces
// the code on screen, and the last module that loaded stays until the next one does. A module that throws as it draws is told in the body and does not take the pane with it.

import * as React from "react";
import * as ui from "@armada/components";

import { useModList } from "../mod-list";
import "./tab-host.css";
import { loadTabModule, type TabComponent, type TabHost } from "./tab-module";
import { TAB_GLYPHS } from "./tab-glyph";

type Loaded = { Component: TabComponent | undefined; /** Which load it came from. */ load: number; problems: readonly string[] };

/** The module of the mod called `name`, reloaded when its row's `changed_at` moves. Not loaded while the mod is off or not valid. */
export function useTabModule(name: string): Loaded {
  const row = useModList().find((one) => one.name === name);
  const stamp = `${row?.changed_at ?? ""}:${row?.valid === true && row.enabled}`;
  const [loaded, setLoaded] = React.useState<Loaded>({ Component: undefined, load: 0, problems: [] });
  React.useEffect(() => {
    if (row === undefined) return;
    let live = true;
    const fail = (problems: readonly string[]) => live && setLoaded((was) => ({ ...was, problems }));
    void (async () => {
      if (!row.enabled) return fail(["Switched off"]);
      if (!row.valid) return fail([row.reason ?? `${name} is not valid`]);
      const checked = await window.armada.validateMod(name);
      if (checked === null) return fail(["Fleet did not answer"]);
      if (!checked.valid || checked.tab === undefined) return fail(checked.problems.length === 0 ? [`${name} is not valid`] : checked.problems);
      try {
        const Component = await loadTabModule(checked.tab.code);
        if (live) setLoaded((was) => ({ Component, load: was.load + 1, problems: [] }));
      } catch (why) {
        fail([why instanceof Error ? why.message : String(why)]);
      }
    })();
    return () => {
      live = false;
    };
    // The row is read for what it said when `stamp` last moved.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [name, stamp]);
  return loaded;
}

class Contained extends React.Component<{ children: React.ReactNode }, { failed: string | undefined }> {
  override state = { failed: undefined as string | undefined };
  static getDerivedStateFromError(why: unknown) {
    return { failed: why instanceof Error ? why.message : String(why) };
  }
  override render() {
    return this.state.failed === undefined ? this.props.children : <p className="job-custom__fault" role="alert">{this.state.failed}</p>;
  }
}

export function TabMod({ mod, jobId, tabId }: { mod: string; jobId: string; tabId: string }) {
  const { Component, load, problems } = useTabModule(mod);
  const host = React.useMemo<TabHost>(() => ({ React, armada: window.armada, ui, glyphs: TAB_GLYPHS, job: { id: jobId }, tab: { id: tabId } }), [jobId, tabId]);
  if (Component === undefined) {
    return problems.length === 0 ? null : (
      <ul className="job-custom__faults" role="alert">
        {problems.map((one) => (
          <li key={one}>{one}</li>
        ))}
      </ul>
    );
  }
  // A reloaded module is another component, so the old one's state goes with it; an error it threw does too.
  return (
    <Contained key={load}>
      <Component host={host} />
    </Contained>
  );
}
