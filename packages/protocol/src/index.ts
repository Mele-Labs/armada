// The Fleet wire, in TypeScript, and the one thing every other package may
// import.
//
// **This package imports nothing.** It is what Fleet sends and what a read of
// it looks like, so a screen can render a Session without knowing that an Electron
// preload carried it — which is the rule that lets Screens sit above Bridge
// rather than inside it.
//
// Half of it is generated. `apps/desktop/codegen/vocabulary.mjs` writes
// `generated/` from the Rust domain registries under
// `crates/core-model/domain/`, which stay the authority: a status, a verb or a
// glyph is decided there and rendered here.

export * from "./accounts";
export * from "./amending";
export * from "./clashes";
export * from "./commanding";
export * from "./committing";
export * from "./sharing";
export * from "./connection";
export * from "./drift";
export * from "./editing";
export * from "./events";
export * from "./files";
export * from "./flight-controls";
export * from "./folding";
export * from "./forge";
export * from "./health";
export * from "./helm";
export * from "./helm-calls";
export * from "./helm-debug";
export * from "./thought";
export * from "./retro";
export * from "./retro-review";
export * from "./retro-session";
export * from "./logbook";
export * from "./holding";
export * from "./kit";
export * from "./building";
export * from "./limits";
export * from "./logbook";
export * from "./manifest-proposal";
export * from "./merge-lines";
export * from "./mods";
export * from "./teaching";
export * from "./preferences";
export * from "./reach-folders";
export * from "./settings";
export * from "./pending";
export * from "./protocol";
export * from "./reading";
export * from "./reads";
export * from "./requester";
export * from "./requests";
export * from "./rehearsal";
export * from "./scan";
export * from "./pull-requests";
export * from "./pull-reviews";
export * from "./review-runs";
export * from "./review-walks";
export * from "./issues";
export * from "./autopilot";
export * from "./servers";
export * from "./hosted-sessions";
export * from "./sessions";
export * from "./session-notes";
export * from "./session-mcp";
export * from "./context-breakdown";
export * from "./loadout";
export * from "./turn-usage";
export * from "./usage-rules";
export * from "./usage-summary";
export * from "./session-activity";
export * from "./session-build";
export * from "./session-plan";
export * from "./session-rail";
export * from "./session-waits";
export * from "./plan-usage";
export * from "./model-downshift";
export * from "./sleep";
export * from "./setup";
export * from "./session-import";
export * from "./run-memory";
export * from "./notices";
export * from "./studio";
export * from "./territory";
export * from "./timeline";
export * from "./act-failed";
export * from "./bay-history";
export * from "./triggers";
export * from "./underway";
export * from "./watches";
