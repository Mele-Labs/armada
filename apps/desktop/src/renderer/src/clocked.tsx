// A screen that draws ages takes the time as a prop. This gives it one from the window's clock, read
// here, so a tick draws that screen and not the `App` above it that wired it.

import type { ComponentType } from "react";
import { useNow } from "@armada/components";

export function clocked<P extends { now?: number }>(Component: ComponentType<P>): ComponentType<Omit<P, "now">> {
  function Clocked(props: Omit<P, "now">) {
    return <Component {...(props as P)} now={useNow()} />;
  }
  Clocked.displayName = `Clocked(${Component.displayName ?? Component.name})`;
  return Clocked;
}
