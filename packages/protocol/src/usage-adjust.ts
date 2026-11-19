// The settings that change the burn, what each would save, and what each change did, mirrored by hand from
// `crates/ipc/src/usage_adjust.rs`. `now`, `suggested` and every choice's `value` are what `save_settings`
// takes for `key`; a `label` is what a person reads. A share is 0 to 1 of the window's whole burn.

import type { SettingValue } from "./settings";

/** One value a setting may be changed to. */
export type AdjustChoice = {
  value: SettingValue;
  label: string;
};

/** A setting the page changes in place. */
export type AdjustSetting = {
  /** The settings.json path, which `save_settings` takes as the key of a change. */
  key: string;
  label: string;
  /** What the file holds now, or what ships where it holds none. */
  now: SettingValue;
  /** The values the page offers, always with `now` among them. */
  choices: AdjustChoice[];
  /** The value the window's requests say would save the most. Absent where none would, or where Fleet cannot say. */
  suggested?: SettingValue;
  /** The id of the waste kind it cuts, as `get_usage_waste` names it. */
  cuts: string;
  /** The share of the window's burn a change to `suggested` would save. Absent where Fleet cannot estimate it. */
  saves_share?: number;
};

/** A change to one of those settings, and the waste it cuts before and since. */
export type AdjustChange = {
  key: string;
  label: string;
  /** What it was and what it became, as a person reads them: `5 min`, `1 hour`. */
  from: string;
  to: string;
  /** When the change came into force. */
  at: string;
  cuts: string;
  /** The `cuts` kind's share of the burn over the seven days before `at`. */
  before_share: number;
  /** The same share from `at` to now. Absent until a day has passed. */
  since_share?: number;
  /** The days from `at` to now, which is how much `since_share` has to go on. */
  since_days: number;
};

/** `get_usage_adjustments` (`GET /usage/adjustments?window=`). */
export type UsageAdjustments = {
  settings: AdjustSetting[];
  /** Newest first, of the last thirty days. */
  changes: AdjustChange[];
};
