// The one seam a prototype body mounts through: the mock sets `cockpitBody.current` before the app
// renders, and the app never does. Nothing in the production bundle imports a prototype.
// proto/cockpit-three-directions.

import type { ComponentProps, ComponentType } from "react";

import type { Cockpit } from "./Cockpit";

export type CockpitProps = ComponentProps<typeof Cockpit>;

export const cockpitBody: { current: ComponentType<CockpitProps> | undefined } = { current: undefined };
