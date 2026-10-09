// The Dashboard's keys beyond the Board's: switching its filters, and the sheet that lists them all.
// Bare keys follow the contextual tier (`holdsText` suppresses them in a field, a modifier means
// another tier); the one modified binding, Option and a digit, is a filter picked by number.
//
// **Every key is read from the keymap**, so the sheet and the handlers answer what Settings → Keyboard
// says this person pressed, and nothing here spells a key the registry already owns.

import { useState } from "react";
import { Kbd, Sheet, actionOf, formatSlot, isPressed, keyFor, pressedDigit, pressedSlot, slotsOf } from "@armada/components";
import { DASHBOARD_TABS, type DashboardTab } from "@armada/overview";
import { useLayout } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

/** Every key an act answers now, one per slot, for drawing as caps. */
const capsOf = (id: string): string[] => slotsOf(id).map(formatSlot).filter((one) => one !== "");

/** Previous and next filter, and the filter by number with Option held — as this person has them. */
export function tabKeys(): { previous: string; next: string; numbered: string } {
  const [previous, next] = slotsOf("dashboard_filters").map(formatSlot);
  return { previous: previous ?? "", next: next ?? "", numbered: keyFor("dashboard_filter_number") };
}

type Group = { head: string; rows: readonly { keys: readonly string[]; does: string }[] };

/** What the sheet lists, read when it is drawn. The words are the acts' own: the registry's where it has one. */
function groups(): readonly Group[] {
  const tabs = tabKeys();
  const [down, up, downArrow, upArrow] = slotsOf("move_focus").map(formatSlot);
  return [
    {
      head: "A call in front",
      rows: [
        { keys: capsOf("call_pick"), does: actionOf("call_pick").verb },
        { keys: [down ?? "", up ?? ""].filter((one) => one !== ""), does: "Move between answers" },
        { keys: [...capsOf("call_best"), ...capsOf("call_quick")], does: `${actionOf("call_best").verb}, ${actionOf("call_quick").verb.toLowerCase()}` },
        { keys: capsOf("call_send"), does: actionOf("call_send").verb },
        { keys: [...capsOf("call_later"), "Esc"], does: actionOf("call_later").verb },
        { keys: capsOf("call_dismiss"), does: `${actionOf("call_dismiss").verb}, never to show again` },
        { keys: capsOf("open"), does: "Open what it is about" },
        { keys: capsOf("call_expand"), does: actionOf("call_expand").verb },
        { keys: capsOf("call_reply"), does: actionOf("call_reply").verb },
      ],
    },
    {
      head: "The fleet",
      rows: [
        { keys: [down, up, "←", "→", upArrow, downArrow].filter((one): one is string => one !== undefined && one !== ""), does: actionOf("move_focus").verb },
        { keys: [...capsOf("open_focused"), ...capsOf("open")], does: actionOf("open").verb },
        { keys: capsOf("kill"), does: actionOf("kill").verb },
        { keys: capsOf("call_recall"), does: actionOf("call_recall").verb },
        { keys: ["Esc", "↓"], does: "Leave the request field" },
      ],
    },
    {
      head: "Dashboard",
      rows: [
        { keys: [tabs.previous, tabs.next].filter((one) => one !== ""), does: actionOf("dashboard_filters").verb },
        { keys: capsOf("dashboard_filter_number"), does: actionOf("dashboard_filter_number").verb },
        { keys: capsOf("dashboard_view"), does: actionOf("dashboard_view").verb },
        { keys: capsOf("new_job"), does: actionOf("new_job").verb },
        { keys: capsOf("key_sheet"), does: actionOf("key_sheet").verb },
      ],
    },
  ];
}

/** The filters in the layout's order, less those it hides; a hidden one still shows while it is the one on. */
export function useFilters(tab: DashboardTab): { id: DashboardTab; label: string }[] {
  return useLayout("dashboard.tabs")
    .all.filter((one) => one.visible || one.id === tab)
    .map((one) => DASHBOARD_TABS.find((known) => known.id === one.id))
    .filter((one) => one !== undefined);
}

/**
 * `[` and `]` step the filters round, Option and 1 to 3 pick one, `?` opens the key sheet. Bound while
 * the Dashboard is drawn and a field does not have the keys.
 */
export function useDashboardKeys(tab: DashboardTab, onTab: (tab: DashboardTab) => void, listening: boolean): { sheet: boolean; closeSheet: () => void } {
  const [sheet, setSheet] = useState(false);
  const tabs = useFilters(tab);
  useListKeydown((event) => {
    if (!listening || holdsText(event.target)) return;
    const at = tabs.findIndex((one) => one.id === tab);
    // Option changes the character a digit types; the keymap reads the key by its place on the keyboard.
    const digit = pressedDigit("dashboard_filter_number", event);
    if (digit !== null) {
      const picked = tabs[digit - 1];
      if (picked === undefined) return;
      event.preventDefault();
      onTab(picked.id);
      return;
    }
    if (event.repeat) return;
    const step = pressedSlot("dashboard_filters", event);
    if (step !== -1) {
      event.preventDefault();
      onTab(tabs[(at + (step === 1 ? 1 : -1) + tabs.length) % tabs.length]!.id);
    } else if (isPressed("key_sheet", event)) {
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
        {groups().map((group) => (
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
