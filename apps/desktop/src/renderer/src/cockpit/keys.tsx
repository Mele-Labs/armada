// The Dashboard's keys beyond the Board's: switching its filters, and the sheet that lists them all.
// Bare keys follow the contextual tier (`holdsText` suppresses them in a field, a modifier means
// another tier); the one modified binding, Option and a digit, is a filter picked by number.
//
// **Proposed, not registered.** `[` `]` `?` `l` `w` and the number keys are in no row of
// `actions.toml`, so no caption may read them from the registry yet; they are written here, once, and
// move there if the owner keeps them. Everything the registry owns — move, open, kill — is read from it.

import { useState } from "react";
import { Kbd, Sheet, actionOf, keyFor } from "@armada/components";
import { DASHBOARD_TABS, type DashboardTab } from "@armada/overview";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

/** Previous and next filter, and the filter by number with Option held: the registry's own spellings. */
const [PREVIOUS, NEXT] = keyFor("dashboard_filters").split(" ") as [string, string];
export const TAB_KEYS = { previous: PREVIOUS, next: NEXT, numbered: keyFor("dashboard_filter_number") } as const;

type Group = { head: string; rows: readonly { keys: readonly string[]; does: string }[] };

/** What the sheet lists. The words are the acts' own: the registry's where it has one. */
const GROUPS: readonly Group[] = [
  {
    head: "A call in front",
    rows: [
      { keys: [keyFor("call_pick")], does: actionOf("call_pick").verb },
      { keys: ["j", "k"], does: "Move between answers" },
      { keys: [keyFor("call_best"), keyFor("call_quick")], does: `${actionOf("call_best").verb}, ${actionOf("call_quick").verb.toLowerCase()}` },
      { keys: ["↵"], does: actionOf("call_send").verb },
      { keys: [keyFor("call_later"), "Esc"], does: actionOf("call_later").verb },
      { keys: [keyFor("open")], does: "Open what it is about" },
      { keys: [keyFor("call_expand")], does: actionOf("call_expand").verb },
    ],
  },
  {
    head: "The fleet",
    rows: [
      { keys: ["j", "k", "←", "→", "↑", "↓"], does: actionOf("move_focus").verb },
      { keys: ["↵", keyFor("open")], does: actionOf("open").verb },
      { keys: [keyFor("kill")], does: actionOf("kill").verb },
      { keys: [keyFor("call_recall")], does: actionOf("call_recall").verb },
      { keys: ["Esc", "↓"], does: "Leave the request field" },
    ],
  },
  {
    head: "Dashboard",
    rows: [
      { keys: [TAB_KEYS.previous, TAB_KEYS.next], does: actionOf("dashboard_filters").verb },
      { keys: [TAB_KEYS.numbered], does: actionOf("dashboard_filter_number").verb },
      { keys: [keyFor("dashboard_view")], does: actionOf("dashboard_view").verb },
      { keys: [keyFor("new_job")], does: actionOf("new_job").verb },
      { keys: [keyFor("key_sheet")], does: actionOf("key_sheet").verb },
    ],
  },
];

/**
 * `[` and `]` step the filters round, Option and 1 to 3 pick one, `?` opens the key sheet. Bound while
 * the Dashboard is drawn and a field does not have the keys.
 */
export function useDashboardKeys(tab: DashboardTab, onTab: (tab: DashboardTab) => void, listening: boolean): { sheet: boolean; closeSheet: () => void } {
  const [sheet, setSheet] = useState(false);
  useListKeydown((event) => {
    if (!listening || event.metaKey || event.ctrlKey || holdsText(event.target)) return;
    const at = DASHBOARD_TABS.findIndex((one) => one.id === tab);
    // Option changes the character a digit types, so the key is read by its place on the keyboard.
    const digit = /^Digit([1-9])$/.exec(event.code)?.[1];
    if (event.altKey) {
      const picked = digit === undefined ? undefined : DASHBOARD_TABS[Number(digit) - 1];
      if (picked === undefined) return;
      event.preventDefault();
      onTab(picked.id);
      return;
    }
    if (event.repeat) return;
    if (event.key === PREVIOUS || event.key === NEXT) {
      event.preventDefault();
      onTab(DASHBOARD_TABS[(at + (event.key === NEXT ? 1 : -1) + DASHBOARD_TABS.length) % DASHBOARD_TABS.length]!.id);
    } else if (event.key === keyFor("key_sheet")) {
      event.preventDefault();
      setSheet(true);
    }
  });
  return { sheet, closeSheet: () => setSheet(false) };
}

/** Every key the Dashboard answers, as a sheet over the work area. */
export function KeySheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Sheet open={open} title="Keys" kind="dashboard-keys" floating closeBinding="Esc" onClose={onClose}>
      <div className="armada-keys">
        {GROUPS.map((group) => (
          <section key={group.head} className="armada-keys__group" aria-label={group.head}>
            <h3 className="armada-keys__head">{group.head}</h3>
            {group.rows.map((row) => (
              <dl key={row.does} className="armada-keys__row">
                <dt>
                  {row.keys.map((key) => (
                    <Kbd key={key}>{key}</Kbd>
                  ))}
                </dt>
                <dd>{row.does}</dd>
              </dl>
            ))}
          </section>
        ))}
      </div>
    </Sheet>
  );
}
