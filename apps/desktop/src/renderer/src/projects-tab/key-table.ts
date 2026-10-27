// The Projects tab's keys, **read off the registry** (`projects_*` rows of `actions.toml`, scope `projects`), so Settings →
// Keyboard moves a key and the caps, the tooltips and the handlers in `keys.tsx` follow. Nothing here spells a key.

import { formatSlot, slotsOf } from "@armada/components";

const capOfSlot = (id: string, at: number): string => {
  const slot = slotsOf(id)[at];
  return slot === undefined ? "" : formatSlot(slot);
};

export const PROJECTS_KEYS = {
  get list() {
    return capOfSlot("projects_panel", 0);
  },
  get rows() {
    return capOfSlot("projects_panel", 1);
  },
  get newProject() {
    return capOfSlot("projects_new", 0);
  },
  get addTask() {
    return capOfSlot("projects_add_task", 0);
  },
  get addSubtask() {
    return capOfSlot("projects_add_subtask", 0);
  },
  get remove() {
    return capOfSlot("projects_delete", 0);
  },
  get edit() {
    return capOfSlot("projects_edit", 0);
  },
  get openJob() {
    return capOfSlot("projects_open_job", 0);
  },
  get unlinkJob() {
    return capOfSlot("projects_unlink_job", 0);
  },
  get unlinkPull() {
    return capOfSlot("projects_unlink_pull", 0);
  },
  get rename() {
    return capOfSlot("projects_rename", 0);
  },
  get select() {
    return capOfSlot("projects_select", 0);
  },
  get tick() {
    return capOfSlot("projects_tick", 0);
  },
  get dispatch() {
    return capOfSlot("projects_dispatch", 0);
  },
  get markDone() {
    return capOfSlot("projects_mark_done", 0);
  },
  get keepOpen() {
    return capOfSlot("projects_keep_open", 0);
  },
  get group() {
    return capOfSlot("projects_group", 0);
  },
  /** The key of the view at `at`: 0 all, 1 needs you, 2 in a Job. */
  view: (at: number): string => capOfSlot("projects_view", at),
};

export { capOf } from "../pr-review/key-table";
