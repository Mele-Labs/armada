// The Fleet wire, in TypeScript, and the one thing every other package may
// import.
//
// **This package imports nothing.** It is what Fleet sends and what a read of
// it looks like, so a screen can render a Job without knowing that an Electron
// preload carried it — which is the rule that lets Screens sit above Bridge
// rather than inside it.
//
// Half of it is generated. `apps/desktop/codegen/vocabulary.mjs` writes
// `generated/` from the Rust domain registries under
// `crates/core-model/domain/`, which stay the authority: a status, a verb or a
// glyph is decided there and rendered here.

export * from "./acts";
export * from "./amending";
export * from "./approving";
export * from "./artifacts";
export * from "./asking";
export * from "./attempt";
export * from "./commanding";
export * from "./confidence";
export * from "./connection";
export * from "./detail";
export * from "./drift";
export * from "./drones";
export * from "./editing";
export * from "./events";
export * from "./explaining";
export * from "./files";
export * from "./folding";
export * from "./footprint";
export * from "./forge";
export * from "./generated/protocol-version";
export * from "./health";
export * from "./helm";
export * from "./helm-calls";
export * from "./helm-debug";
export * from "./helm-thread";
export * from "./history";
export * from "./retro";
export * from "./holding";
export * from "./journal";
export * from "./judged";
export * from "./kit";
export * from "./limits";
export * from "./manifest-proposal";
export * from "./merge-lines";
export * from "./preferences";
export * from "./proposal";
export * from "./pending";
export * from "./protocol";
export * from "./proposing";
export * from "./reading";
export * from "./reads";
export * from "./reclaimed";
export * from "./manifest-checks";
export * from "./requester";
export * from "./rehearsal";
export * from "./scan";
export * from "./needs";
export * from "./servers";
export * from "./sessions";
export * from "./remarks";
export * from "./report";
export * from "./resources";
export * from "./setup";
export * from "./showing";
export * from "./studio";
export * from "./walk-notes";
export * from "./turn";
export * from "./underway";
export * from "./version";
export * from "./waiting";
export * from "./work";
export * from "./work-plan";
