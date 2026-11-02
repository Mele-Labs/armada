// A built Custom tab's body: the tab mod's module, run in place. It is loaded again each time Fleet's list says the mod's files changed, so what the Mod Job writes replaces
// the code on screen, and the last module that loaded stays until the next one does. A module that throws as it draws is told in the body and does not take the pane with it.
// Each fault but a switched-off mod offers Fix, a Mod Job on the tab told the reason, which the tab then follows.

import * as React from "react";
import { Button } from "@armada/components";

import { refusal } from "../annotate/send";
import { useModList } from "../mod-list";
import { askToTell } from "../tell";
import { followJob, tabTitle } from "./custom";
import "./tab-host.css";
import { loadTabModule, type TabComponent, type TabHost } from "./tab-module";
import { TAB_GLYPHS } from "./tab-glyph";
import { TAB_UI } from "./tab-ui";

type Loaded = {
  Component: TabComponent | undefined /** Which load it came from. */;
  load: number;
  problems: readonly string[] /** Switched off, which is his choice and nothing to fix. */;
  off: boolean;
};

/** The module of the mod called `name`, reloaded when its row's `changed_at` moves. Not loaded while the mod is off or not valid. */
export function useTabModule(name: string): Loaded {
  const row = useModList().find((one) => one.name === name);
  const stamp = `${row?.changed_at ?? ""}:${row?.valid === true && row.enabled}`;
  const [loaded, setLoaded] = React.useState<Loaded>({
    Component: undefined,
    load: 0,
    problems: [],
    off: false,
  });
  React.useEffect(() => {
    if (row === undefined) return;
    let live = true;
    const fail = (problems: readonly string[]) =>
      live &&
      setLoaded((was) => ({
        ...was,
        problems,
        off: false,
      }));
    // Off or invalid is Fleet's word on the mod, so the module it last ran goes too; a load that failed keeps it.
    const refuse = (problems: readonly string[], off = false) =>
      live &&
      setLoaded((was) => ({
        Component: undefined,
        load: was.load,
        problems,
        off,
      }));
    void (async () => {
      if (!row.enabled)
        return refuse(
          [
            "Switched off",
          ],
          true,
        );
      if (!row.valid)
        return refuse([
          row.reason ?? `${name} is not valid`,
        ]);
      const checked = await window.armada.validateMod(name);
      if (checked === null)
        return fail([
          "Fleet did not answer",
        ]);
      if (!checked.valid || checked.tab === undefined)
        return refuse(
          checked.problems.length === 0
            ? [
                `${name} is not valid`,
              ]
            : checked.problems,
        );
      try {
        const Component = await loadTabModule(checked.tab.code);
        if (live)
          setLoaded((was) => ({
            Component,
            load: was.load + 1,
            problems: [],
            off: false,
          }));
      } catch (why) {
        fail([
          why instanceof Error ? why.message : String(why),
        ]);
      }
    })();
    return () => {
      live = false;
    };
    // The row is read for what it said when `stamp` last moved.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    name,
    stamp,
  ]);
  return loaded;
}

/** Start a Mod Job on the tab, told why it is broken, and have the tab follow it. A refusal is told as a toast. */
async function fix(
  mod: string,
  jobId: string,
  tabId: string,
  problems: readonly string[],
): Promise<void> {
  const title = tabTitle(mod);
  const said = `Fix the ${title} tab: change the tab mod \`${mod}\` in place, without scaffolding another.\n\n${problems.join("\n")}`;
  const started = await window.armada.startModSession({
    title: `Mod Fix ${title}`,
    said,
    aim: "tab",
    from_job: jobId,
    from_tab: tabId,
  });
  if (!started.ok) return askToTell(`Mod Job not started: ${refusal(started.outcome)}`);
  followJob(tabId, started.value.session_id);
}

function Faults({
  problems,
  fixing,
  over,
}: {
  problems: readonly string[] /** Absent where there is nothing to fix. */;
  fixing?: () => Promise<void> /** Drawn above the last module that loaded. */;
  over?: boolean;
}) {
  const [pending, setPending] = React.useState(false);
  return (
    <div className="job-custom__faults" role="alert" data-over={over || undefined}>
      <ul className="job-custom__fault-list">
        {problems.map((one) => (
          <li key={one}>{one}</li>
        ))}
      </ul>
      {fixing === undefined ? null : (
        <Button
          size="sm"
          pending={pending}
          onClick={() => (setPending(true), void fixing().finally(() => setPending(false)))}
        >
          Fix
        </Button>
      )}
    </div>
  );
}

class Contained extends React.Component<
  {
    children: React.ReactNode;
    fault: (why: string) => React.ReactNode;
  },
  {
    failed: string | undefined;
  }
> {
  override state = {
    failed: undefined as string | undefined,
  };
  static getDerivedStateFromError(why: unknown) {
    return {
      failed: why instanceof Error ? why.message : String(why),
    };
  }
  override render() {
    return this.state.failed === undefined
      ? this.props.children
      : this.props.fault(this.state.failed);
  }
}

export function TabMod({ mod, jobId, tabId }: { mod: string; jobId: string; tabId: string }) {
  const { Component, load, problems, off } = useTabModule(mod);
  const host = React.useMemo<TabHost>(
    () => ({
      React,
      armada: window.armada,
      ui: TAB_UI,
      glyphs: TAB_GLYPHS,
      job: {
        id: jobId,
      },
      tab: {
        id: tabId,
      },
    }),
    [
      jobId,
      tabId,
    ],
  );
  const fixing = (said: readonly string[]) => () => fix(mod, jobId, tabId, said);
  if (Component === undefined) {
    return problems.length === 0 ? null : (
      <Faults
        problems={problems}
        {...(off
          ? {}
          : {
              fixing: fixing(problems),
            })}
      />
    );
  }
  // A reloaded module is another component, so the old one's state goes with it; an error it threw does too. A rewrite that did not load is told above the one still drawn.
  return (
    <>
      {problems.length === 0 ? null : <Faults problems={problems} fixing={fixing(problems)} over />}
      <Contained
        key={load}
        fault={(why) => (
          <Faults
            problems={[
              why,
            ]}
            fixing={fixing([
              why,
            ])}
          />
        )}
      >
        <Component host={host} />
      </Contained>
    </>
  );
}
