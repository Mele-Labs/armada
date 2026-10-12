// A Job's walk window opening or taking focus makes its Job Bridge's open Job — the owner's call of
// 11 Oct 2026, `docs/practices/capture-window.md`, *The panel beside the page*.

import { CHANNELS, type Summons } from "../../shared/bridge";
import type { LandsOn } from "./layout";

/**
 * The Job a walk window moves Bridge to, or `null`. **A Studio's window moves nothing**, and nor does a
 * Session's: its panel is live on its own, and Bridge has no one open Session for it to take.
 */
export function followedBy(landsOn: LandsOn): string | null {
  return "job" in landsOn ? landsOn.job.id : null;
}

export type Following = {
  /** Main's own `watchJob`: the open Job read whole and kept current, which the panel draws from. */
  watch: (jobId: string) => void;
  /** Bridge's own window, where one is open. */
  bridge: () => Pick<Electron.WebContents, "send"> | null;
};

/**
 * **Watched here, and told to Bridge's window as a summons.** The watch is what the panel draws when
 * Bridge's window is closed or has the composer up; the summons is what keeps that window's own open
 * Job the same one, so main never swaps the watch out from under what it shows. Nothing is raised:
 * the walk window keeps the focus it just took.
 */
export function follow(jobId: string, { watch, bridge }: Following): void {
  watch(jobId);
  const told: Summons = { jobId, follow: true };
  bridge()?.send(CHANNELS.summoned, told);
}
