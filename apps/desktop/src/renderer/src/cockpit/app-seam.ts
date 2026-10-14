// The one seam a navigation prototype mounts through, beside `body-seam.ts`: the mock sets `appSeam` before the
// app renders, and the app never does, so nothing here changes the production app's behaviour. A chrome
// component the shell draws at the top of its mount on every surface, the Work group left off the rail, and
// where Fleet's call toasts give way to it. proto/cockpit-three-directions.

import type { ComponentProps, ComponentType } from "react";

import type { Cockpit } from "./Cockpit";

/** What the chrome is given: the Cockpit's own props, and where the window is. */
export type AppSeamProps = ComponentProps<typeof Cockpit> & {
  /** The rail row the window is on: `SURFACE.overview` for the Cockpit. */
  showing: string;
  /** The Job open on the Cockpit's surface, or none. */
  openJobId: string | null;
  /** The Session open on the Sessions page, or none. */
  openSessionId: string | null;
  goTo: (surface: string) => void;
  /** One place back, as `⌘[` goes. */
  back: () => void;
  /** Goes to a place in place of the layers over the Cockpit, so Back still returns to the Cockpit. */
  replace: (surface: string) => void;
};

export const appSeam: { current: ComponentType<AppSeamProps> | undefined; hideWork: boolean } = { current: undefined, hideWork: false };
