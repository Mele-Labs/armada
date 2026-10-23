# packages/mods/

Mods that ship built in. Each folder is one mod as `promote_mod` writes it:
`mod.toml` and its `theme.css`, `layout.json` or `tracker.toml`.

Fleet compiles every folder here in (`crates/fleet/build.rs`) and lists it as
`built-in:<folder>`, beside any mod of the same name in a machine's own mods
folder. It is checked by the rules a local mod is, and one that breaks a rule
is listed with its problem and never applied. `docs/concepts/mods.md`.

This file keeps the folder in every checkout, so the build always has it to
watch.
