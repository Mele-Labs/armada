// The pages under System, as a strip of tabs under the title bar. There only while System is the place, and
// nothing otherwise, so the window mounts it unconditionally. #2301.

import { Tabs } from "@armada/components";
import { deckTabOf } from "@armada/shell";

import { systemPagesOf } from "./system-page";
import "./deck-chrome.css";

export function SystemStrip({ showing, onGoTo, hidden = [], warned = {} }: {
  /** The surface that is up. */
  showing: string;
  /** Opens a surface. */
  onGoTo: (surface: string) => void;
  /** Surfaces left off the strip: by a layout, or because nothing serves them. */
  hidden?: readonly string[];
  /** A mark on a page, by surface id, and what its tooltip says. */
  warned?: Readonly<Record<string, string>>;
}) {
  if (deckTabOf(showing) !== "system") return null;
  return (
    <div className="deck-chrome">
      <nav className="deck-system" aria-label="System">
        <Tabs
          items={systemPagesOf(hidden).map((page) => ({ id: page.surface, label: page.label, ...(warned[page.surface] === undefined ? {} : { alert: warned[page.surface]! }) }))}
          value={showing}
          onChange={onGoTo}
        />
      </nav>
    </div>
  );
}
