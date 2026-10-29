// The panel's shell for what is not a pull request: a head, a title with its facts and acts under it, and tabs, drawn as
// the pull request panel draws them so the four read as one. The pull request keeps `PullRequestPanel`, whose head this is.

import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import { Sheet, TabPanel, Tabs } from "@armada/components";
import type { SheetBack } from "@armada/components";
import { useAtFloor } from "@armada/shell";

type Tab = { id: string; label: string };

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/**
 * `[` and `]` move to the tab before and after, wherever focus is in the panel. **Held on the window while the panel is up**, so it
 * answers with focus on the close button where the panel opens it. Not while a box is being typed in.
 */
export function useTabKeys(ids: readonly string[], tab: string, onTab: (id: string) => void): void {
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      const by = event.key === "]" ? 1 : event.key === "[" ? -1 : 0;
      if (by === 0 || ids.length === 0) return;
      // A dialog over the panel (a box that asks first) keeps its own keys.
      if (document.querySelector(".armada-dialog") !== null) return;
      event.preventDefault();
      const at = Math.max(0, ids.indexOf(tab));
      onTab(ids[(at + by + ids.length) % ids.length]!);
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [ids, tab, onTab]);
}

export function SidePanel({
  title,
  repository,
  heading,
  crumb,
  beside,
  meta,
  acts,
  bar,
  tabs,
  tab,
  onTab,
  back,
  onClose,
  left,
  children,
}: {
  /** `Issue #1873`: names the dialog. */
  title: string;
  repository?: string;
  heading: string;
  /** What the heading's thing sits in, drawn over it: a request's collection. */
  crumb?: string | undefined;
  /** An act on the heading's thing, on the heading's line. */
  beside?: ReactNode;
  meta?: ReactNode;
  acts?: ReactNode;
  /** A control over the tabs that every tab answers to. */
  bar?: ReactNode;
  /** Absent, the panel is the one body and draws no tabs. */
  tabs?: readonly Tab[];
  tab?: string;
  onTab?: (id: string) => void;
  back?: SheetBack | undefined;
  onClose: () => void;
  /**
   * A list floated in a sheet to the panel's left, as the pull request panel floats its files. **Not drawn at the floor**, where
   * it would lie over the panel: the caller draws it in the body there.
   */
  left?: { title: string; body: ReactNode } | undefined;
  /** The chosen tab's body. */
  children: ReactNode;
}) {
  const floor = useAtFloor();
  const body = useRef<HTMLDivElement>(null);
  const floats = left !== undefined && !floor;
  useTabKeys(
    (tabs ?? []).map((one) => one.id),
    tab ?? "",
    onTab ?? (() => undefined),
  );
  // The list's close takes focus as it opens; the tab keeps it, as in the pull request panel.
  useEffect(() => {
    if (floats) body.current?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus({ preventScroll: true });
  }, [floats]);
  return (
    <>
    <Sheet
      kind="side-panel"
      open
      floating
      under={floats}
      floor={floor}
      title={title}
      {...(repository === undefined ? {} : { subtitle: <span className="armada-pr-panel__repository mono">{repository}</span> })}
      size="panel"
      {...(back === undefined ? {} : { back })}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-pr-panel" ref={body}>
        {crumb === undefined && beside === undefined ? (
          <h3 className="armada-pr-panel__title">{heading}</h3>
        ) : (
          <div className="side-panel__head">
            <div className="side-panel__heading">
              {crumb === undefined ? null : <p className="side-panel__crumb">{crumb}</p>}
              <h3 className="armada-pr-panel__title">{heading}</h3>
            </div>
            {beside}
          </div>
        )}
        {meta === undefined ? null : <div className="armada-pr-panel__meta">{meta}</div>}
        {acts === undefined ? null : (
          <div className="armada-pr-panel__acts" role="group" aria-label="Acts">
            {acts}
          </div>
        )}
        {bar === undefined ? null : <div className="side-panel__bar">{bar}</div>}
        {tabs === undefined || tab === undefined || onTab === undefined ? (
          <div className="armada-pr-tabs">
            <div className="armada-pr-tabs__panel">{children}</div>
          </div>
        ) : (
          <div className="armada-pr-tabs">
            <Tabs items={[...tabs]} value={tab} onChange={onTab} />
            <TabPanel tab={tab} role="tabpanel" aria-label={tabs.find((one) => one.id === tab)?.label} className="armada-pr-tabs__panel">
              {children}
            </TabPanel>
          </div>
        )}
      </div>
    </Sheet>
    {floats && left !== undefined ? (
      /* The panel's own, so its close and Esc close the panel. */
      <Sheet open floating beside="side-panel" kind="side-panel-list" size="panel" title={left.title} closeBinding="Esc" onClose={onClose}>
        {left.body}
      </Sheet>
    ) : null}
    </>
  );
}
