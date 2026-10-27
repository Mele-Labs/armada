// Holding a macOS banner until it is dealt with.
//
// **Electron collects a `Notification` nobody references**, and a collected one never fires "click": pressing
// the banner only activates the app. Pressing one later from Notification Center makes that near certain.

/** Anything shaped like Electron's `Notification`: what is held and shown. */
export type Banner = {
  // One overload to an event, as Electron types them: a union of events would not accept `Notification`.
  on(event: "click", listener: () => void): unknown;
  on(event: "close", listener: () => void): unknown;
  on(event: "failed", listener: () => void): unknown;
  show(): void;
};

const held = new Set<Banner>();

/** Show a banner and keep it until it is pressed, closed or fails. */
export function showHeld(banner: Banner): void {
  held.add(banner);
  const letGo = (): void => {
    held.delete(banner);
  };
  banner.on("click", letGo);
  banner.on("close", letGo);
  banner.on("failed", letGo);
  banner.show();
}

/** Whether a banner is still held. */
export function isHeld(banner: Banner): boolean {
  return held.has(banner);
}
