// Whether this Bridge is a dev build, for what only a dev build turns on: the annotation layer, the notes
// watch, the sounds install.
//
// **`app.isPackaged` alone is not it.** Electron reads it off the executable's name, and the dev copy
// `scripts/armada-electron` makes is named Armada, so a dev build read as packaged and lost all of them.
// A dev build is still started through Electron's default app with the source folder, which sets
// `process.defaultApp`; a packaged one never is.

/** True for a dev build: not packaged, or packaged in name only and started with a source folder. */
export function isDevBuild(packaged: boolean, defaultApp: boolean | undefined): boolean {
  return !packaged || defaultApp === true;
}
