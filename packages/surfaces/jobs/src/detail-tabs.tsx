// Job detail's eight destinations, and the strip that chooses between them.
//
// **One label per tab, written once.** The design boards call the same
// destination "Plan", "Plan the split" and "Plan 4 groups" in three places, and
// a screen that repeats a name is a screen where two of them drift.

// **Settings is the sixth, and it was a button until 28 September 2026.** The
// owner reversed his own 21 September decision knowingly: *"It should be part
// of the segment control where overview, workflow, plan, record, and pulse are
// now."*

// **Underline tabs, not the segmented control.** Every Overview artboard draws
// the destinations as plain text with a rule under the active one. Two boxed
// controls stacked — the destinations over a panel's own filters — have no
// hierarchy between them, and the board buys it with two kinds of control at
// two levels. `Tabs` stays filled, because that is what a panel's filters are.

// **A destination carries a figure, or says nothing.** `Start-4-Running` draws
// `Workflow 1 / 5` and `Epic-Overview` draws `Record 21`, so the trailing value
// is mono and subtle beside the name rather than a chip.

import type { JobDetail, JobSummary } from "@armada/protocol";
import { useLayout } from "@armada/shell";

import { changedOf, offersSettings } from "./settings";

/**
 * The eight destinations, in the order the strip draws them.
 *
 * **Checks after Record, and with no figure** (the owner, 7 Oct 2026): the Record keeps its Check
 * rows, and this is the same Checks as the Checks page reads them, for this Job alone. A figure
 * beside it would count the rows it lists.
 *
 * **Drones after Record, and with no figure** (the owner, 29 Sep 2026): Pulse
 * already counts the Drones running, and the same number twice is one too many.
 *
 * **Settings last, because it is the only one that is not a reading.** The
 * six before it answer what this Job is and what it did; this one changes it.
 */
export const DETAIL_TABS = [
  "overview",
  "workflow",
  "plan",
  "record",
  "checks",
  "drones",
  "pulse",
  "settings",
] as const;

export type DetailTab = (typeof DETAIL_TABS)[number];

/** Where a reader lands, and what `every-state` opens on: today's arrangement. */
export const FIRST_TAB: DetailTab = "overview";

/**
 * The one spelling of each tab's name. Sentence case, one word each — the
 * noun the journey doc names the destination by.
 */
export const TAB_LABEL: Record<DetailTab, string> = {
  overview: "Overview",
  workflow: "Workflow",
  plan: "Plan",
  record: "Record",
  checks: "Checks",
  drones: "Drones",
  pulse: "Pulse",
  settings: "Settings",
};

/**
 * What a tab has behind it, where it has a number at all.
 *
 * Read off the Job Fleet answered with, never off the Board's row: the row
 * carries neither the frozen workflow's steps nor the plan's tasks, so a Job
 * whose detail has not arrived draws no counts rather than wrong ones.
 *
 * **Settings counts what somebody changed**, which is the figure the header's
 * button carried before the strip took it over — a Job nobody has touched
 * draws no count, exactly as the button drew none.
 */
export function countsOf(
  whole: JobDetail | null,
  job?: JobSummary,
): Partial<Record<DetailTab, number>> {
  if (whole === null) return {};
  return {
    workflow: whole.steps.length,
    // Tasks a Drone may still do. A dropped task is not work outstanding, and
    // counting it would make the figure climb as a plan is pruned.
    ...(whole.work_plan === undefined
      ? {}
      : { plan: whole.work_plan.tasks.filter((task) => task.state !== "dropped").length }),
    ...(job !== undefined && offersSettings(job, whole) ? { settings: changedOf(whole) } : {}),
  };
}

export type JobTabsProps = {
  value: DetailTab;
  onChange: (tab: DetailTab) => void;
  counts: Partial<Record<DetailTab, number>>;
};

/**
 * The strip under the Job header. **The whole of navigation inside a Job**,
 * which is why it carries its own name: there is no heading beside it to be
 * one, and a reader who cannot see it would otherwise hear eight tabs and never
 * what they divide.
 */
export function JobTabs({ value, onChange, counts }: JobTabsProps) {
  // The layout's order and what it hides; a hidden tab still shows while it is the one open.
  const layout = useLayout("job.tabs");
  const tabs = layout.all.filter((one) => one.visible || one.id === value).map((one) => one.id as DetailTab);
  function move(step: number): void {
    const at = tabs.indexOf(value);
    const next = tabs[(at + step + tabs.length) % tabs.length];
    if (next !== undefined) onChange(next);
  }
  return (
    <div
      className="armada-destinations"
      role="tablist"
      aria-label="Job detail"
      onKeyDown={(event) => {
        if (event.key === "ArrowRight") {
          event.preventDefault();
          move(1);
        }
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          move(-1);
        }
      }}
    >
      {tabs.map((tab) => {
        const count = counts[tab];
        return (
          <button
            key={tab}
            type="button"
            role="tab"
            className="armada-destinations__tab"
            aria-selected={tab === value}
            tabIndex={tab === value ? 0 : -1}
            onClick={() => onChange(tab)}
          >
            {TAB_LABEL[tab]}
            {/* Zero is no figure: a destination with nothing behind it reads
                as a destination, not as one holding a zero. */}
            {count === undefined || count === 0 ? null : (
              <span className="armada-destinations__count">{count}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
