// The text of Copy debug report, as a mock hands it: names, versions and states, no message text, the home folder
// as `~`. Laid out as the error payload is (`errors/ErrorNotice/payload.ts`): one string, no fences, labels
// aligned three spaces past the longest. Invented repository and Jobs.

const GAP = 3;

/** A block's rows aligned on its longest label; a value's further lines sit under its first. */
function block(heading: string, rows: [string, string][]): string {
  const width = Math.max(...rows.map(([label]) => label.length)) + GAP;
  const lines = rows.map(([label, value]) => {
    const [head, ...rest] = value.split("\n");
    return [`  ${label.padEnd(width)}${head}`, ...rest.map((line) => `  ${" ".repeat(width)}${line}`)].join("\n");
  });
  return [heading, ...lines].join("\n");
}

const TAIL = "bridge protocol 5.2  fleet protocol 5.2  taken 2026-11-02T09:31:12Z";

const INSTALL = block("Install", [
  ["armada", "0.14.2"],
  ["runs from", "~/Applications/Armada.app/Contents/Resources/bin/armada"],
  ["armada resolves", "no"],
  ["claude code", "2.1.310"],
]);

const TROUBLE = block("Recent trouble", [
  ["09:12:41", "WARN  door       reconnect_refused"],
  ["09:12:44", "WARN  hosted     init_without_mod"],
  ["09:20:03", "WARN  store      slow_write"],
]);

const MOD = block("Mod", [
  ["fleet copy", "filled"],
  ["fleet copy version", "0.14.2"],
  ["plugin installed", "0.14.1"],
]);

const REPOSITORY = block("This repository  web-app", [
  ["armada.yml", "present"],
  ["kept in", "Kit"],
]);

const HEALTHY = block("Job  Add retry to the sync worker", [
  ["id", "4f2a91c0"],
  ["runs", "hosted"],
  ["mod", "0.14.2"],
  ["door", "connected"],
  ["missing", "nothing"],
  ["tools", "Bash, Edit, Glob, Grep, Read, Write\nmcp__armada-fleet__waiting_for, mcp__armada-fleet__start_another_job"],
  ["skills", "armada:reading-a-brief, armada:verify-before-done"],
]);

const NO_MOD = block("Job  Fix the flaky checkout test", [
  ["id", "b81d03e7"],
  ["runs", "terminal"],
  ["mod", "none"],
  ["door", "connected"],
  ["missing", "mod, armada: skills"],
  ["tools", "Bash, Edit, Glob, Grep, Read, Write"],
  ["skills", "none"],
]);

/** Fleet answered: every part. */
export const REPORT = ["Armada debug report", INSTALL, MOD, REPOSITORY, HEALTHY, NO_MOD, TROUBLE, TAIL].join("\n\n");

/** Fleet did not answer: what Bridge reads itself, and the line saying so. */
export const REPORT_FLEET_SILENT = ["Armada debug report", INSTALL, "Fleet   did not answer", TROUBLE, TAIL].join("\n\n");
