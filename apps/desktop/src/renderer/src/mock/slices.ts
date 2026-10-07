// Every surface a fake is made of, in one list. A surface's fake data joins by one line here, and
// a whole-app test that wants only core and its own surface mounts with that list (`fakeBridge`).

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import type { Slice, SliceName } from "./fake-context";
import { cleanup } from "./slices/cleanup";
import { core } from "./slices/core";
import { helm } from "./slices/helm";
import { jobs } from "./slices/jobs";
import { manifest } from "./slices/manifest";
import { overview } from "./slices/overview";
import { reports } from "./slices/reports";
import { sessions } from "./slices/sessions";
import { settings } from "./slices/settings";
import { setup } from "./slices/setup";
import { studios } from "./slices/studios";
import { triggers } from "./slices/triggers";
import { workflows } from "./slices/workflows";

/** A slice of any surface: what `fake.ts` composes, once each member is known to be answered. */
export type AnySlice = Slice<Partial<BridgeApi>, Partial<BridgeState>>;

/** **A `Record` over every name**, so a surface with no entry here fails typecheck. */
const BY_NAME = { core, studios, manifest, setup, workflows, triggers, helm, settings, overview, cleanup, reports, jobs, sessions } satisfies Record<
  SliceName,
  AnySlice
>;

/** Core first, then each surface. */
export const SLICES: readonly AnySlice[] = Object.values(BY_NAME);

export type { SliceName };

type ApiOf<T> = T extends Slice<infer A, unknown> ? A : never;
type Intersection<U> = (U extends unknown ? (one: U) => void : never) extends (all: infer I) => void ? I : never;

/** Fails typecheck where the slices together answer less than `BridgeApi` names. */
export type EverySliceTogetherIsBridgeApi = Intersection<ApiOf<(typeof BY_NAME)[SliceName]>> extends BridgeApi ? true : never;
const _complete: EverySliceTogetherIsBridgeApi = true;
void _complete;

/** The moments a surface's own Fleet makes, for the slices named. */
export function scenariosOf(...names: SliceName[]) {
  return names.flatMap((name) => BY_NAME[name].scenarios?.() ?? []);
}
