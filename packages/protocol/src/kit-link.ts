// Kit over the Mac link, mirrored by hand from `crates/ipc/src/kit_link.rs`: what Bridge reads and the acts it
// sends. What one Mac offers another crosses between Fleets and Gateways only, and is not mirrored.
// `docs/concepts/kit.md`, *Over the Mac link*.

import type { HomeItem, HomeKind, HomeKindSwitch, HomeUnreadable } from "./shared-home";
import type { ModKind } from "./mods";

/** How a row stands against one linked Mac. `differs` crosses on the next exchange; `clash` waits for his pick. */
export type KitMark = "same" | "differs" | "new_here" | "only_there" | "clash";

/** One Mac's version of an item, as a clash or a difference shows it. `changed_at` is when that Mac first read it. */
export type KitSide = {
  mac_name: string;
  text: string;
  changed_at?: string;
};

/** One row against one linked Mac, by its id. A row that differs or clashes carries both versions. */
export type KitAgainst = {
  mac: string;
  mark: KitMark;
  here?: KitSide;
  there?: KitSide;
};

/** One Kit item that crosses, or would. `refused` says why this Mac's version stays here, naming a host and never a value. */
export type KitLinkRow = {
  kind: HomeKind;
  name: string;
  repository?: string;
  says?: string;
  source?: string;
  key_here?: boolean;
  mod_kind?: ModKind;
  refused?: string;
  against: KitAgainst[];
};

/** A linked Mac Kit crosses to: when its Kit last crossed here, and what it has switched on. */
export type KitLinkedMac = {
  mac: string;
  name: string;
  crossed_at?: string;
  kinds: HomeKind[];
  items: HomeItem[];
};

/** `GET /kit/link`, and what every act on it answers. */
export type KitLink = {
  kinds: HomeKindSwitch[];
  items: HomeItem[];
  macs: KitLinkedMac[];
  rows: KitLinkRow[];
  unreadable: HomeUnreadable[];
};

/** `POST /kit/link/kinds/set`. */
export type SetKitLinkKind = {
  kind: HomeKind;
  on: boolean;
};

/** `POST /kit/link/items/set`: the item's fields beside `on`. */
export type SetKitLinkItem = HomeItem & {
  on: boolean;
};

/** Which side of a clash is kept, on both Macs. */
export type KitKeep = "here" | "there";

/** `POST /kit/link/settle`: the item's fields beside the linked Mac and the side kept. */
export type SettleKitClash = HomeItem & {
  mac: string;
  keep: KitKeep;
};
