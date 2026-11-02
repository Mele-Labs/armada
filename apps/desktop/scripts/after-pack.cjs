// electron-builder's `afterPack`: refuses to pack an Armada that would ship without its mod, because a Job on a Mac
// with only the app would come up with no skills, subagents or hooks. `crates/adapters/src/agent_mod.rs` reads it.

const { existsSync } = require("node:fs");
const { join } = require("node:path");

const MOD_FILES = [".claude-plugin/marketplace.json", "armada/.claude-plugin/plugin.json"];

exports.default = async function afterPack(context) {
  const app = `${context.packager.appInfo.productFilename}.app`;
  const mod = join(context.appOutDir, app, "Contents", "Resources", "mod");
  for (const file of MOD_FILES) {
    if (!existsSync(join(mod, file))) throw new Error(`The mod did not reach the app: ${join(mod, file)} is missing.`);
  }
};
