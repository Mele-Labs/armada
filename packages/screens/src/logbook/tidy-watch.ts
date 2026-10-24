// The Tidy pass this window has pressed and not yet reported: the `last` Fleet held at the press, so a Logbook page
// opened again later can tell the pass from an old one. Module-level so it outlives the page.

export type TidyWatch = { last?: string };

let held: TidyWatch | undefined;

export const tidyWatch = (): TidyWatch | undefined => held;
export const watchTidy = (watch: TidyWatch): void => {
  held = watch;
};
/** Cleared once the result is shown. */
export const forgetTidyWatch = (): void => {
  held = undefined;
};
