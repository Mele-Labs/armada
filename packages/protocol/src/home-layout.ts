// The Cockpit home's layout, one per user, kept by Fleet as `bridge.homeLayout`. Mirrors `crates/ipc/src/home_layout.rs`.
// The blocks are the project board's own, so Bridge draws both with one grid.

import type { BoardLayout } from "./project-board";

/** `GET` and `POST /home/layout`: `layout` is absent where he saved none, and Bridge draws its preset. */
export type HomeLayout = {
  layout?: BoardLayout;
};

/** `POST /home/layout`: the layout to keep, whole. Absent forgets it. */
export type SaveHomeLayout = {
  layout?: BoardLayout;
};
