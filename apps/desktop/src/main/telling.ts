// The sound a notification carries, by the tone it is told in.
//
// **Permission is macOS's, and Armada does not build a second one.** The first notification is what
// makes macOS ask, once, and the answer is the person's and is remembered by the system. A refusal
// costs nothing here: `show()` becomes a no-op and every other part of Bridge is untouched.

/** `blocked` is a person's call to make; `waiting` is a thing that has come up. */
export type Tone = "blocked" | "waiting";

/**
 * The sound name a notification carries for each tone: `sounds/<name>.wav`,
 * copied into the bundle's Resources by `electron-builder.yml`.
 *
 * **A name, never a player.** macOS plays it as the notification's own sound,
 * so permission, Focus and per-app sound settings mute it with the banner.
 */
const SOUNDS: Record<Tone, string> = {
  blocked: "armada-blocked",
  waiting: "armada-waiting",
};

export function soundOf(tone: Tone): string {
  return SOUNDS[tone];
}
