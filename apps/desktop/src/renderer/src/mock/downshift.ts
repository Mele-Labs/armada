// Mock-only: the weekly window's downshift offer with no Fleet behind it. It reads the plan's windows the
// title bar reads, offers the lower model once per window at or over the line, and moves the Session default by
// saving `models.session` the way the Settings page does, so the composer and Settings follow.

import type { SettingValue } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import type { Downshift, DownshiftSource } from "../downshift";
import type { PlanUsageSource } from "../plan-usage";

/** The weekly figure from which the offer is made. */
const LINE = 80;
/** The model the default steps down to. */
const LOWER = "sonnet";
const KEY = "models.session";

export function createMockDownshift(usage: PlanUsageSource, api: BridgeApi): DownshiftSource {
  let state: Downshift | undefined;
  /** The reset time of the weekly window last read, and the one an offer was last made for. */
  let window: string | undefined;
  let offeredFor: string | undefined;
  /** What `models.session` held in settings.json before the step down: `null` is not saved, so shipped. */
  let saved: SettingValue | null = null;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: Downshift | undefined): void => {
    state = to;
    listeners.forEach((on) => on());
  };

  const sessionSetting = async () => (await api.state()).settings?.settings.find((one) => one.key === KEY);

  const read = (): void => {
    const weekly = usage.get()?.windows.find((one) => one.kind === "seven_day");
    if (weekly === undefined) return;
    const reset = weekly.resets_at;
    const fresh = window !== undefined && reset !== window;
    window = reset;
    if (fresh && state?.phase === "shifted") return set({ phase: "reset", was: state.was, to: state.to });
    if (fresh && state?.phase === "offered") set(undefined);
    if (state?.phase === "offered") return set({ ...state, percent: weekly.percent_used });
    if (state !== undefined || weekly.percent_used < LINE || offeredFor === reset) return;
    offeredFor = reset;
    void sessionSetting().then((one) => {
      const was = typeof one?.value === "string" ? one.value : "opus";
      if (was !== LOWER) set({ phase: "offered", percent: weekly.percent_used, was, to: LOWER });
    });
  };

  return {
    get: () => state,
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) {
        stop = usage.subscribe(read);
        read();
      }
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    accept: () => {
      const offer = state;
      if (offer?.phase !== "offered" || window === undefined) return;
      const until = window;
      void sessionSetting().then(async (one) => {
        saved = one?.saved ?? null;
        await api.saveSettings({ [KEY]: offer.to });
        set({ phase: "shifted", was: offer.was, to: offer.to, until });
      });
    },
    restore: () => {
      void api.saveSettings({ [KEY]: saved }).then(() => set(undefined));
    },
    keep: () => set(undefined),
    settle: () => {
      offeredFor = window;
      set(undefined);
    },
  };
}
