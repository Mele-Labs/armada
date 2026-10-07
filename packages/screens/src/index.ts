// A whole screen, assembled, and the readings behind it.
//
// **Props in, callbacks out.** Nothing here opens a socket, reaches for a
// preload or knows what Electron is: a screen decides what the wire means and
// what a person may do about it, and the app it is mounted in does the asking.
// That is what lets a screen be rendered, storied and tested with no daemon.
//
// The host calls a screen needs arrive as arguments — `onReadDiff`,
// `onOpenArtifact`, `onNeedMaterial`, `onStage`, `onWant`. Each
// used to be a `window.armada` call written inline, which is precisely what
// held these files inside the app.

export * from "./Report";
export * from "./Reports";
export * from "./Row";
export * from "./landed-words";
export * from "./Taken";
export * from "./freeze";
// A repository's Studios, and one open on its whiteboard. #1287.
export * from "./studio";
export * from "./studio-frames";
export * from "./studio-reads";
export * from "./title";
export * from "./board";
// The merge line `armada land` keeps, as Fleet serves it. Since 22.1.
export * from "./merge-line";
// On All repositories, the question a surface that needs one repository asks first.
export * from "./AskRepository";
export * from "./copy";
export * from "./editing";
// Every question waiting on a person, from every repository, as Helm's dock draws them.
export * from "./dock-questions";
export * from "./refusal-words";
export * from "./outstanding";
export * from "./duration";
export * from "./facts";
export * from "./frozen";
export * from "./held";
export * from "./keys";
export * from "./manifest-kit";
export * from "./verify";
export * from "./opening";
export * from "./origin";
export * from "./reading";
export * from "./drawn-patch";
export * from "./waiting";
export * from "./open-studio";
// Setup and Locate are `@armada/setup`; their reads stay here, since main, the wire types and Manifest read them.
export type * from "./setup-reads";
export * from "./locate-reads";
// Overview's reads stay here, since Settings and the wire types read them; the surface is `@armada/overview`.
export type * from "./overview-reads";
export * from "./pausing";
