// The vocabulary Bridge renders with, emitted from the files that own it.
//
// It is written twice over: as TypeScript for the Electron Bridge, and as Rust for the native
// Bridge (`crates/bridge-native/src/generated/`). Both come from the one reading below.
//
// `crates/core-model/domain/enum-verbs.toml` is the authority on how a variant
// reads, `check-outcomes.toml` on whether a Check outcome advances a step, and
// `actions.toml` on what every act is called and bound to.
// All of them are read here and written into TypeScript, because the
// alternative — a verb list, a glyph map or a literal typed into a component —
// is the second
// vocabulary that drifted three times before it was deleted.
//
// **`actions.toml` is here because it had been transcribed by hand three
// times** — into the contract's key map, into `packages/screens/src/keys.ts`,
// and into a 592-line `packages/components/src/actions.ts` that landed and was
// deleted the same week. A registry with three copies has three answers the
// day one of them is edited alone. The contract's copy stays and is held to the
// registry by `xtask/src/rules_actions.rs`; the two TypeScript copies are now
// this file's output.
//
// This is a stopgap in the honest sense: `crates/ipc/src/lib.rs` says a codegen
// step emits TypeScript from the Rust types, and when that lands it should
// absorb this script rather than sit beside it. The precedent for a Node
// generator in this repository is `packages/components/gallery/build.mjs`.
//
// Run it with `pnpm --filter @armada/desktop codegen`. The output is checked in,
// and `cargo xtask verify-foundations` compares it against a fresh run through
// `--emit` — see the bottom of this file, and `xtask/src/rules_vocabulary.rs`.

import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");
const VERBS = join(repo, "crates", "core-model", "domain", "enum-verbs.toml");
const OUTCOMES = join(repo, "crates", "core-model", "domain", "check-outcomes.toml");
const ACTS = join(repo, "crates", "core-model", "domain", "actions.toml");
// The vocabulary is a rendering and not a wire type: it carries the glyph each
// variant draws as, so it imports `lucide-react`. It belongs with the
// components that render it, and `@armada/protocol` stays dependency-free.
const OUT = join(repo, "packages", "components", "src", "generated", "vocabulary.ts");
// Beside the vocabulary and not inside it: a variant is a state something is in and
// an act is something a person does to it, and the two are separate registries
// with separate gates. They share a destination for the same reason the
// vocabulary is there at all — an act draws a glyph, and a glyph is React.
const OUT_ACTIONS = join(repo, "packages", "components", "src", "generated", "actions.ts");
// The native Bridge reads the same registries through the same joins, so its copy is this script's
// output as well and the gate compares it the same way. `crates/bridge-native` is its own Cargo
// workspace, which is why these are checked-in files and not a `build.rs` of its own: the gate
// that holds the TypeScript to the registries holds these too.
const OUT_RUST_VOCABULARY = join(repo, "crates", "bridge-native", "src", "generated", "vocabulary.rs");
const OUT_RUST_ACTIONS = join(repo, "crates", "bridge-native", "src", "generated", "actions.rs");

// The vocabularies a surface renders. `criterion_verdict_*` and `evidence_type` are
// not: a Check's verdict and a step's evidence were a Job's rail, and no surface
// draws them now.
//
// `step_state` and `advance_gate` were wanted here before either had a row, on
// purpose: asking for a vocabulary is what puts the gap in this script's own
// output — `advance_gate (no rows in enum-verbs.toml)` — rather than leaving a
// renderer to write the missing words itself.
const WANTED = [
  "queued_reason",
  "check_outcome",
  "step_state",
  // A task of a plan.
  "task_state",
  // A group of a plan, between a step and a task. It has no enum behind it;
  // `enum-verbs.toml`'s header says why, and its glyphs are the step's borrowed
  // one level further down.
  "group_state",
  // A branch on the merge line, as Bridge's Overview draws it.
  "land_state",
  "advance_gate",
  // The Setup sheet and the Manifest form both offer its three words, and both showed the file's.
  "auto_merge",
  "change_kind",
];

// A glyph the registry names and this lucide-react version does not export.
// Carried as data rather than fixed by a rename, because the rename is the
// registry's to make: `packages/icons/icons/` decides what a glyph means,
// and a generator quietly substituting a different export would be deciding it
// here instead. The variant renders without a glyph and is counted in `GAPS`.
const NOT_EXPORTED = new Set([
  "file-question",
]);

/// The one shape these files have: table headers, `key = value`, `#` comments,
/// no multi-line strings. The same line parser `xtask/src/rules_icons.rs`
/// justifies for the same reason — not general TOML, and it does not pretend to
/// be. A line it cannot read stops the build.
function tables(text, path) {
  const found = new Map();
  let current = null;
  text.split("\n").forEach((raw, i) => {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) return;
    if (line.startsWith("[")) {
      if (!line.endsWith("]")) throw new Error(`${path}:${i + 1} — unreadable table header`);
      current = line.slice(1, -1);
      found.set(current, {});
      return;
    }
    const split = line.indexOf("=");
    if (split < 0) throw new Error(`${path}:${i + 1} — neither a header nor a key`);
    const key = line.slice(0, split).trim();
    const value = line.slice(split + 1).trim();
    const table = current === null ? null : found.get(current);
    if (table === null) throw new Error(`${path}:${i + 1} — a key outside every table`);
    table[key] = unquoted(value);
  });
  return found;
}

/// A basic string, a literal string, or a bare token left as it is.
///
/// **The literal form is read because one binding needs it.** `⌘\` is Toggle
/// sidebar's, and a trailing backslash cannot be written in a TOML basic string
/// without escaping — so `actions.toml` spells that one row `'⌘\'`. Every other
/// value in every registry this script reads is double-quoted, which is why
/// adding this changed nothing already emitted.
function unquoted(value) {
  for (const quote of [
    '"',
    "'",
  ]) {
    if (value.length >= 2 && value.startsWith(quote) && value.endsWith(quote)) {
      return value.slice(1, -1);
    }
  }
  return value;
}

/** `user-check` becomes `UserCheck`, which is what lucide-react exports it as. */
function pascal(kebab) {
  return kebab
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

/** A TypeScript string literal, or `null`. */
function quoted(value) {
  return value === null || value === "" ? "null" : JSON.stringify(value);
}

/** A Rust string literal. JSON's escapes are not Rust's for a control character, so they are written out. */
function rustStr(value) {
  let out = '"';
  for (const ch of value) {
    if (ch === "\\") out += "\\\\";
    else if (ch === '"') out += '\\"';
    else if (ch === "\n") out += "\\n";
    else if (ch === "\r") out += "\\r";
    else if (ch === "\t") out += "\\t";
    else if (ch.codePointAt(0) < 0x20) out += `\\u{${ch.codePointAt(0).toString(16)}}`;
    else out += ch;
  }
  return `${out}"`;
}

/** `Some("x")`, or `None` for a blank: a blank is a gap in the registry, never a default. */
function rustOption(value) {
  return value === null || value === "" ? "None" : `Some(${rustStr(value)})`;
}

/** `list and detail` becomes `ListAndDetail`. */
function rustVariant(words) {
  return words
    .split(/[\s_-]+/)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

const verbs = tables(readFileSync(VERBS, "utf8"), "enum-verbs.toml");

// `advances` per Check outcome, from the registry that owns the set.
//
// Emitted because a surface has to tell a Check that stopped a step from one
// that did not, and the two are not the same question as passed. `skipped`
// advances and is not a pass; `never_ran` neither advances nor passes, and both
// carry --status-not-started — so a rule read off the status token gets one of
// them wrong. The registry already answers it; Bridge reads that answer rather
// than writing a second one.
const advances = new Map();
{
  let current = null;
  for (const raw of readFileSync(OUTCOMES, "utf8").split("\n")) {
    const line = raw.trim();
    const header = /^\[outcomes\.([a-z_]+)\]$/.exec(line);
    if (header !== null) {
      current = header[1];
      advances.set(current, null);
      continue;
    }
    if (current === null) continue;
    const flag = /^advances = (true|false)$/.exec(line);
    if (flag !== null) advances.set(current, flag[1] === "true");
  }
}
for (const [outcome, flag] of advances) {
  if (flag === null)
    throw new Error(`check-outcomes.toml — [outcomes.${outcome}] names no advances`);
}

const vocabularies = new Map(
  WANTED.map((name) => [
    name,
    [],
  ]),
);
const gaps = [];
const glyphs = new Set();

for (const [header, table] of verbs) {
  const parts = header.split(".");
  if (parts[0] !== "verbs" || parts.length !== 3) continue;
  // `unquoted`, because a wire value TOML cannot spell bare is written quoted
  // and is still the same value: `advance_gate`'s two policy forms carry a
  // colon. Emitting the quotes would key the map by a string no wire value
  // equals, so every such variant would render as its own raw spelling — the
  // exact failure this generated map exists to end. A quoted key holding a dot
  // would be split into four parts and dropped by the guard above; none does,
  // and one would fail `xtask`'s pairing rule loudly rather than silently.
  const [, vocabulary, variant] = [
    parts[0],
    parts[1],
    unquoted(parts[2]),
  ];
  const rows = vocabularies.get(vocabulary);
  if (rows === undefined) continue;

  const verb = table.verb ?? "";
  const icon = table.icon ?? "";
  const token = table.status_token ?? "";
  const hint = table.hint ?? "";
  const missing = [];
  if (verb === "") missing.push("verb");
  if (icon === "") missing.push("icon");
  else if (NOT_EXPORTED.has(icon)) missing.push("icon");
  if (token === "") missing.push("token");
  if (missing.length > 0)
    gaps.push({
      vocabulary,
      variant,
      missing,
      icon,
    });

  const usable = icon !== "" && !NOT_EXPORTED.has(icon);
  if (usable) glyphs.add(icon);
  // Badge takes the token stem and spells `--status-{stem}` and
  // `--status-{stem}-bg` itself, so the stem is derived from the token rather
  // than from the variant name — `queued` renders `--status-not-started`.
  const stem = token.startsWith("--status-") ? token.slice("--status-".length) : "";
  rows.push({
    variant,
    verb,
    icon: usable ? icon : "",
    glyph: icon,
    token,
    stem,
    hint,
  });
}

const imported = [
  ...glyphs,
]
  .sort()
  .map(pascal);

const lines = [];
lines.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
lines.push("//");
lines.push("// The verb, the glyph and the status token each variant renders as, from");
lines.push("// `crates/core-model/domain/enum-verbs.toml`; whether a Check outcome");
lines.push("// advances a step, from `check-outcomes.toml`. Nothing here is written by");
lines.push("// hand, which is the point: a state label typed into a component is a");
lines.push("// second vocabulary.");
lines.push("//");
lines.push("// A `null` verb, glyph or token is a gap in the registry, not a default. Every");
lines.push("// one is listed in `GAPS` so a surface can say what it could not render instead");
lines.push("// of inventing copy for it.");
lines.push("");
lines.push(`import { ${imported.join(", ")} } from "lucide-react";`);
lines.push('import type { LucideIcon } from "lucide-react";');
lines.push("");
lines.push("/** How one variant reads. `null` where the registry carries no answer. */");
lines.push("export type Rendering = {");
lines.push("  readonly verb: string | null;");
lines.push("  readonly icon: LucideIcon | null;");
lines.push("  /** Badge's `status` prop: the token stem, without its `--status-` prefix. */");
lines.push("  readonly badgeStatus: string | null;");
lines.push("  readonly statusToken: string | null;");
lines.push("  /**");
lines.push("   * The sentence a tooltip reads for this variant, where the verb alone");
lines.push("   * gives no reasoning. `null` where the registry carries none — most");
lines.push("   * tables do not, and a surface renders no tooltip rather than one with");
lines.push("   * an empty label.");
lines.push("   */");
lines.push("  readonly hint: string | null;");
lines.push("};");
lines.push("");

for (const [vocabulary, rows] of vocabularies) {
  const constant = vocabulary.toUpperCase();
  if (rows.length === 0) {
    lines.push(
      `/** \`${vocabulary}\` — **no rows in \`enum-verbs.toml\`.** Every variant is a gap. */`,
    );
    lines.push(`export const ${constant}: Readonly<Record<string, Rendering | undefined>> = {};`);
    lines.push("");
    continue;
  }
  lines.push(`/** \`${vocabulary}\`, keyed by the wire value. */`);
  // `| undefined` is spelled out rather than left to `noUncheckedIndexedAccess`,
  // because a wire value this build has never heard of is a real answer and a
  // reader of this file should see that without checking a compiler flag.
  lines.push(`export const ${constant}: Readonly<Record<string, Rendering | undefined>> = {`);
  for (const row of rows) {
    const icon = row.icon === "" ? "null" : pascal(row.icon);
    lines.push(
      `  ${JSON.stringify(row.variant)}: { verb: ${quoted(row.verb)}, icon: ${icon}, ` +
        `badgeStatus: ${quoted(row.stem)}, statusToken: ${quoted(row.token)}, ` +
        `hint: ${quoted(row.hint)} },`,
    );
  }
  lines.push("};");
  lines.push("");
}

// An outcome in one
// registry and not the other renders with a verb and no rule, or the reverse.
const drawn = new Set((vocabularies.get("check_outcome") ?? []).map((row) => row.variant));
for (const outcome of drawn) {
  if (!advances.has(outcome)) throw new Error(`check-outcomes.toml has no [outcomes.${outcome}]`);
}
for (const outcome of advances.keys()) {
  if (!drawn.has(outcome)) {
    throw new Error(`enum-verbs.toml has no [verbs.check_outcome.${outcome}]`);
  }
}

lines.push("/** A variant the registry has no sanctioned copy, glyph or hue for. */");
lines.push("export type Gap = {");
lines.push("  readonly vocabulary: string;");
lines.push("  readonly variant: string;");
lines.push("  readonly missing: readonly string[];");
lines.push("};");
lines.push("");
lines.push("export const GAPS: readonly Gap[] = [");
for (const gap of gaps) {
  lines.push(
    `  { vocabulary: ${JSON.stringify(gap.vocabulary)}, variant: ${JSON.stringify(gap.variant)}, ` +
      `missing: [${gap.missing.map((m) => JSON.stringify(m)).join(", ")}] },`,
  );
}
lines.push("];");
lines.push("");

const vocabularyModule = lines.join("\n");

// ------------------------------------------------------------ vocabulary, in Rust
//
// The same joins as above for the native Bridge: the registry's glyph is a name in
// `packages/icons/icons/`, and that name is what gpui-kit-assets' lucide set is looked up by. Nothing
// here is filtered through `NOT_EXPORTED`, which is a fact about `lucide-react`; a glyph the native
// set lacks is found by the unit test that checks every name against it.
const rustGaps = [];
const rv = [];
rv.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
rv.push("//");
rv.push("// The verb, the glyph and the status token each variant renders as, from");
rv.push("// `crates/core-model/domain/enum-verbs.toml`. The TypeScript copy is");
rv.push("// `packages/components/src/generated/vocabulary.ts`, from the same script.");
rv.push("//");
rv.push("// `None` is a gap in the registry, not a default, and every one is listed in `GAPS`.");
rv.push("");
rv.push("#![allow(dead_code)]");
rv.push("");
rv.push("/// How one variant reads. `None` where the registry carries no answer.");
rv.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
rv.push("pub struct Rendering {");
rv.push("    pub verb: Option<&'static str>,");
rv.push("    /// A name in `packages/icons/icons/`, which is a lucide name.");
rv.push("    pub glyph: Option<&'static str>,");
rv.push("    /// The token stem without its `--status-` prefix.");
rv.push("    pub badge_status: Option<&'static str>,");
rv.push("    /// The token, such as `--status-running`.");
rv.push("    pub status_token: Option<&'static str>,");
rv.push("    /// The sentence a tooltip reads where the verb alone gives no reasoning.");
rv.push("    pub hint: Option<&'static str>,");
rv.push("}");
rv.push("");
for (const [vocabulary, rows] of vocabularies) {
  const constant = vocabulary.toUpperCase();
  rv.push(`/// \`${vocabulary}\`, keyed by the wire value.`);
  rv.push(`pub const ${constant}: &[(&str, Rendering)] = &[`);
  for (const row of rows) {
    const missing = [];
    if (row.verb === "") missing.push("verb");
    if (row.glyph === "") missing.push("icon");
    if (row.token === "") missing.push("token");
    if (missing.length > 0) rustGaps.push({ vocabulary, variant: row.variant, missing });
    rv.push(
      `    (${rustStr(row.variant)}, Rendering { verb: ${rustOption(row.verb)}, ` +
        `glyph: ${rustOption(row.glyph)}, badge_status: ${rustOption(row.stem)}, ` +
        `status_token: ${rustOption(row.token)}, hint: ${rustOption(row.hint)} }),`,
    );
  }
  rv.push("];");
  rv.push("");
}
rv.push("/// Every vocabulary, by the name `enum-verbs.toml` gives it.");
rv.push("pub const VOCABULARIES: &[(&str, &[(&str, Rendering)])] = &[");
for (const vocabulary of vocabularies.keys()) {
  rv.push(`    (${rustStr(vocabulary)}, ${vocabulary.toUpperCase()}),`);
}
rv.push("];");
rv.push("");
rv.push("/// One variant of one vocabulary. `None` where this build has never heard of it.");
rv.push("pub fn rendering(vocabulary: &str, variant: &str) -> Option<&'static Rendering> {");
rv.push("    let (_, rows) = VOCABULARIES.iter().find(|(name, _)| *name == vocabulary)?;");
rv.push("    rows.iter().find(|(wire, _)| *wire == variant).map(|(_, rendering)| rendering)");
rv.push("}");
rv.push("");
rv.push("/// A variant the registry has no sanctioned copy, glyph or hue for.");
rv.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
rv.push("pub struct Gap {");
rv.push("    pub vocabulary: &'static str,");
rv.push("    pub variant: &'static str,");
rv.push("    pub missing: &'static [&'static str],");
rv.push("}");
rv.push("");
rv.push("pub const GAPS: &[Gap] = &[");
for (const gap of rustGaps) {
  rv.push(
    `    Gap { vocabulary: ${rustStr(gap.vocabulary)}, variant: ${rustStr(gap.variant)}, ` +
      `missing: &[${gap.missing.map(rustStr).join(", ")}] },`,
  );
}
rv.push("];");
rv.push("");
const vocabularyRust = rv.join("\n");

// ------------------------------------------------------------------ acts
//
// Every act Bridge offers, from `crates/core-model/domain/actions.toml`.
//
// **This generator validates and does not decide.** `xtask/src/rules_actions.rs`
// is the gate on the registry itself — it holds the glyph column to
// `packages/icons/icons/`, the whole map to the contract's key blocks, and
// the safety rules to the QWERTY layout. What is checked here is narrower and
// is the emitter's own business: that a row says something this script can turn
// into TypeScript without guessing. A row it cannot read stops the build rather
// than reaching a surface as a blank, because a palette entry with no verb is a
// row a person presses and learns nothing from.

const ACTION_KINDS = new Set([
  "Action",
  "Motion",
]);
const ACTION_TIERS = new Set([
  "Global",
  "Contextual",
]);
// Why a glyph may be missing. The registry's two words, and there is no third:
// `undecided` is a gap the icon registry has to close, `by design` is a
// document having closed it with "none". A surface says which it is drawing.
const ABSENCES = new Set([
  "undecided",
  "by design",
]);

const acts = [];
for (const [header, table] of tables(readFileSync(ACTS, "utf8"), "actions.toml")) {
  const parts = header.split(".");
  if (parts[0] !== "actions" || parts.length !== 2) {
    throw new Error(`actions.toml — [${header}] is not an [actions.<id>] table`);
  }
  const id = parts[1];
  const where = `actions.toml — [actions.${id}]`;
  const kind = table.kind ?? "";
  const tier = table.tier ?? "";
  const verb = table.verb ?? "";
  const icon = table.icon ?? "";
  const absent = table.icon_absent ?? "";
  const shortcut = table.shortcut ?? "";
  const scope = table.scope ?? "";
  const unbuilt = table.unbuilt ?? "";

  if (!ACTION_KINDS.has(kind))
    throw new Error(`${where} — kind is "${kind}", not Action or Motion`);
  if (!ACTION_TIERS.has(tier))
    throw new Error(`${where} — tier is "${tier}", not Global or Contextual`);
  if (verb === "") throw new Error(`${where} — no verb, and the palette draws the verb`);
  if (shortcut === "") throw new Error(`${where} — no shortcut, and the palette draws one per row`);
  if (scope === "")
    throw new Error(`${where} — no scope, so no surface can decide whether to offer it`);

  // A Motion appears in no palette, so it carries neither a glyph nor a reason
  // for having none. An Action carries exactly one of the two.
  if (kind === "Motion" && (icon !== "" || absent !== "")) {
    throw new Error(`${where} — a Motion appears in no palette and carries no glyph column`);
  }
  if (icon !== "" && absent !== "") {
    throw new Error(`${where} — a glyph and a reason for having none, and it is one or the other`);
  }
  if (kind === "Action" && icon === "" && !ABSENCES.has(absent)) {
    throw new Error(
      `${where} — no glyph and icon_absent = "${absent}". It is "undecided" or "by design"`,
    );
  }
  // The registry names a glyph this lucide-react cannot export. The vocabulary
  // counts that as a gap because it has a `GAPS` channel to count it in; an act
  // has none, and drawing the row with no glyph would silently contradict a
  // registry that says it has one.
  if (icon !== "" && NOT_EXPORTED.has(icon)) {
    throw new Error(
      `${where} — icon "${icon}" is in the icon registry and lucide-react does not export it`,
    );
  }
  // `unbuilt` is an issue reference and nothing else — the same rule
  // `rules_actions.rs` applies, restated because what is emitted is a number.
  const built = unbuilt === "" ? null : /^#(\d+)$/.exec(unbuilt);
  if (unbuilt !== "" && built === null) {
    throw new Error(`${where} — unbuilt = "${unbuilt}", and it takes an issue reference`);
  }
  for (const [key, value] of [
    [
      "destructive",
      table.destructive,
    ],
    [
      "confirms",
      table.confirms,
    ],
  ]) {
    if (value !== "true" && value !== "false") {
      throw new Error(`${where} — ${key} is "${value ?? ""}", and it is true or false`);
    }
  }

  acts.push({
    id,
    kind,
    tier,
    verb,
    icon,
    absent,
    shortcut,
    scope,
    destructive: table.destructive === "true",
    confirms: table.confirms === "true",
    issue: built === null ? null : Number(built[1]),
  });
}

if (acts.length === 0)
  throw new Error("actions.toml — no [actions.*] table, and Bridge draws them all");

const actGlyphs = [
  ...new Set(acts.map((act) => act.icon).filter((icon) => icon !== "")),
].sort();
// The scopes are read off the rows rather than listed here. A scope is where a
// binding is offered and the registry decides that; a set written into this
// script would be a fourth copy of the thing this file exists to stop having
// four of. What holds a *new* scope to something is the emitted union: the
// palette maps every member of it to a context, so a scope nobody has placed
// fails `pnpm typecheck` with the scope's own name in the message.
const scopes = [
  ...new Set(acts.map((act) => act.scope)),
].sort();

const actLines = [];
actLines.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
actLines.push("//");
actLines.push("// Every act Bridge offers: what it is called, what it is bound to, and the");
actLines.push("// glyph it draws — or why it draws none. From");
actLines.push("// `crates/core-model/domain/actions.toml`, which is the artifact");
actLines.push('// `docs/contracts/design-system.md` promises under "One artifact, three');
actLines.push('// columns" and the authority on all three.');
actLines.push("//");
actLines.push("// **A Motion is here and is not an act.** `move_focus` and `open_focused`");
actLines.push("// move the cursor and act on nothing; the registry says they");
actLines.push("// appear in no palette and carry no glyph, so they are emitted for");
actLines.push("// completeness and filtered out by anything that draws a list of acts.");
actLines.push("//");
actLines.push("// **A blank glyph is a fact, not a default.** `iconAbsent` says which kind of");
actLines.push("// blank it is: `undecided` means no registered silhouette means the act and");
actLines.push("// assigning one is a decision for `packages/icons/icons/`; `by design`");
actLines.push("// means a document decided the act carries none. A surface says which it is");
actLines.push("// drawing rather than inventing a glyph to fill the column.");
actLines.push("//");
actLines.push("// **`unbuilt` names the issue that answers the key.** The registry is ahead");
actLines.push("// of the app deliberately, because the map was settled by drawing. A palette");
actLines.push("// that displays a binding beside every entry would otherwise offer a row a");
actLines.push("// person presses and gets nothing from, which is worse than one that is");
actLines.push("// absent.");
actLines.push("");
if (actGlyphs.length > 0) {
  actLines.push(`import { ${actGlyphs.map(pascal).join(", ")} } from "lucide-react";`);
}
actLines.push('import type { LucideIcon } from "lucide-react";');
actLines.push("");
actLines.push("/** Whether the row is an act or a movement of the cursor. */");
actLines.push('export type ActionKind = "Action" | "Motion";');
actLines.push("");
actLines.push("/** Modifier-based and working anywhere, or single-key and on what is focused. */");
actLines.push('export type ActionTier = "Global" | "Contextual";');
actLines.push("");
actLines.push("/**");
actLines.push(" * Where the binding is offered. The registry's own set, spelled the registry's");
actLines.push(" * way, and read off the rows rather than declared — so a scope that appears");
actLines.push(" * in `actions.toml` appears here and nowhere else has to be told.");
actLines.push(" */");
actLines.push(
  `export type ActionScope =${scopes.map((s) => `\n  | ${JSON.stringify(s)}`).join("")};`,
);
actLines.push("");
actLines.push("/** Why an act's glyph column is empty. `null` where it is not. */");
actLines.push('export type IconAbsence = "undecided" | "by design";');
actLines.push("");
actLines.push("export type Action = {");
actLines.push("  /** The id an implementation binds to. The registry's table key. */");
actLines.push("  readonly id: string;");
actLines.push("  readonly kind: ActionKind;");
actLines.push("  readonly tier: ActionTier;");
actLines.push("  /** What a person reads, in the lexicon's word. Never the id. */");
actLines.push("  readonly verb: string;");
actLines.push("  /** The glyph, or `null` — in which case `iconAbsent` says why. */");
actLines.push("  readonly icon: LucideIcon | null;");
actLines.push("  readonly iconAbsent: IconAbsence | null;");
actLines.push("  /** The binding, spelled as the contract's map spells it. */");
actLines.push("  readonly shortcut: string;");
actLines.push("  readonly scope: ActionScope;");
actLines.push("  readonly destructive: boolean;");
actLines.push("  readonly confirms: boolean;");
actLines.push("  /** The issue that gives the binding an act, on a row nothing answers yet. */");
actLines.push("  readonly unbuilt: string | null;");
actLines.push("};");
actLines.push("");
actLines.push("/**");
actLines.push(" * The map, in the registry's order: global first, then contextual.");
actLines.push(" *");
actLines.push(" * Order is load-bearing in one place only — the palette groups by section and");
actLines.push(" * keeps registry order inside each — so nothing here is sorted.");
actLines.push(" */");
actLines.push("export const ACTIONS: readonly Action[] = [");
for (const act of acts) {
  actLines.push("  {");
  actLines.push(`    id: ${JSON.stringify(act.id)},`);
  actLines.push(`    kind: ${JSON.stringify(act.kind)},`);
  actLines.push(`    tier: ${JSON.stringify(act.tier)},`);
  actLines.push(`    verb: ${JSON.stringify(act.verb)},`);
  actLines.push(`    icon: ${act.icon === "" ? "null" : pascal(act.icon)},`);
  actLines.push(`    iconAbsent: ${quoted(act.absent)},`);
  actLines.push(`    shortcut: ${JSON.stringify(act.shortcut)},`);
  actLines.push(`    scope: ${JSON.stringify(act.scope)},`);
  actLines.push(`    destructive: ${act.destructive},`);
  actLines.push(`    confirms: ${act.confirms},`);
  actLines.push(`    unbuilt: ${act.issue === null ? "null" : JSON.stringify(`#${act.issue}`)},`);
  actLines.push("  },");
}
actLines.push("];");
actLines.push("");
actLines.push("/**");
actLines.push(" * One act, by the id an implementation binds to.");
actLines.push(" *");
actLines.push(" * `| undefined` because a caller may ask for an id this build has never");
actLines.push(" * heard of, and a missing act is a real answer rather than a crash.");
actLines.push(" */");
actLines.push(
  "export const ACTION: Readonly<Record<string, Action | undefined>> = Object.fromEntries(",
);
actLines.push("  ACTIONS.map((action) => [action.id, action]),");
actLines.push(");");
actLines.push("");

const actionsModule = actLines.join("\n");

// ------------------------------------------------------------ acts, in Rust
const ra = [];
ra.push("// GENERATED by `pnpm --filter @armada/desktop codegen`. Do not hand-edit.");
ra.push("//");
ra.push("// Every act Bridge offers: what it is called, what it is bound to, and the glyph it draws or");
ra.push("// why it draws none. From `crates/core-model/domain/actions.toml`. The TypeScript copy is");
ra.push("// `packages/components/src/generated/actions.ts`, from the same script.");
ra.push("//");
ra.push("// A Motion is here and is not an act: it moves the cursor, appears in no palette, and");
ra.push("// `in_palette` is false for it. `unbuilt` names the issue that answers the key, on a row");
ra.push("// Bridge cannot answer yet.");
ra.push("");
ra.push("#![allow(dead_code)]");
ra.push("");
ra.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
ra.push("pub enum ActionKind {");
ra.push("    Action,");
ra.push("    Motion,");
ra.push("}");
ra.push("");
ra.push("/// Modifier-based and working anywhere, or single-key and on what is focused.");
ra.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
ra.push("pub enum ActionTier {");
ra.push("    Global,");
ra.push("    Contextual,");
ra.push("}");
ra.push("");
ra.push("/// Why an act's glyph column is empty.");
ra.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
ra.push("pub enum IconAbsence {");
ra.push("    Undecided,");
ra.push("    ByDesign,");
ra.push("}");
ra.push("");
ra.push("/// Where the binding is offered: the registry's own set, read off the rows.");
ra.push("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]");
ra.push("pub enum ActionScope {");
for (const scope of scopes) ra.push(`    ${rustVariant(scope)},`);
ra.push("}");
ra.push("");
ra.push("impl ActionScope {");
ra.push("    /// The registry's spelling.");
ra.push("    pub const fn as_str(self) -> &'static str {");
ra.push("        match self {");
for (const scope of scopes) ra.push(`            ActionScope::${rustVariant(scope)} => ${rustStr(scope)},`);
ra.push("        }");
ra.push("    }");
ra.push("");
ra.push("    /// The key context a screen sets on the element whose keys these are, such as `list-and-detail`.");
ra.push("    /// `None` for `anywhere`: those keys are bound with no context.");
ra.push("    pub const fn key_context(self) -> Option<&'static str> {");
ra.push("        match self {");
for (const scope of scopes) {
  const context = scope === "anywhere" ? "None" : `Some(${rustStr(scope.replace(/\s+/g, "-"))})`;
  ra.push(`            ActionScope::${rustVariant(scope)} => ${context},`);
}
ra.push("        }");
ra.push("    }");
ra.push("}");
ra.push("");
ra.push("#[derive(Clone, Copy, Debug, PartialEq, Eq)]");
ra.push("pub struct Act {");
ra.push("    /// The id an implementation binds to. The registry's table key.");
ra.push("    pub id: &'static str,");
ra.push("    pub kind: ActionKind,");
ra.push("    pub tier: ActionTier,");
ra.push("    /// What a person reads, in the lexicon's word. Never the id.");
ra.push("    pub verb: &'static str,");
ra.push("    /// A name in `packages/icons/icons/`, or `None`, in which case `icon_absent` says why.");
ra.push("    pub glyph: Option<&'static str>,");
ra.push("    pub icon_absent: Option<IconAbsence>,");
ra.push("    /// The binding, spelled as the contract's map spells it.");
ra.push("    pub shortcut: &'static str,");
ra.push("    pub scope: ActionScope,");
ra.push("    pub destructive: bool,");
ra.push("    pub confirms: bool,");
ra.push("    /// The issue that gives the binding an act, on a row nothing answers yet.");
ra.push("    pub unbuilt: Option<&'static str>,");
ra.push("    /// Whether the command palette lists it: every Action, and no Motion.");
ra.push("    pub in_palette: bool,");
ra.push("}");
ra.push("");
ra.push("/// The map, in the registry's order: global first, then contextual.");
ra.push("pub const ACTIONS: &[Act] = &[");
for (const act of acts) {
  ra.push("    Act {");
  ra.push(`        id: ${rustStr(act.id)},`);
  ra.push(`        kind: ActionKind::${act.kind},`);
  ra.push(`        tier: ActionTier::${act.tier},`);
  ra.push(`        verb: ${rustStr(act.verb)},`);
  ra.push(`        glyph: ${rustOption(act.icon)},`);
  ra.push(
    `        icon_absent: ${act.absent === "" ? "None" : `Some(IconAbsence::${rustVariant(act.absent)})`},`,
  );
  ra.push(`        shortcut: ${rustStr(act.shortcut)},`);
  ra.push(`        scope: ActionScope::${rustVariant(act.scope)},`);
  ra.push(`        destructive: ${act.destructive},`);
  ra.push(`        confirms: ${act.confirms},`);
  ra.push(`        unbuilt: ${act.issue === null ? "None" : rustStr(`#${act.issue}`)},`);
  ra.push(`        in_palette: ${act.kind === "Action"},`);
  ra.push("    },");
}
ra.push("];");
ra.push("");
ra.push("/// One act, by the id an implementation binds to. `None` for an id this build has never heard of.");
ra.push("pub fn act(id: &str) -> Option<&'static Act> {");
ra.push("    ACTIONS.iter().find(|act| act.id == id)");
ra.push("}");
ra.push("");
const actionsRust = ra.join("\n");
// Everything this script writes, in one list rather than a `writeFileSync` per
// destination. The gate reads the same list back through `--emit` and compares
// each entry against what is checked in, so a fourth output added here is gated
// the moment it exists — the alternative is a rule naming three files by hand,
// which is the hole reopened one file over the first time somebody adds a
// fifth.
const EMITTED = [
  [
    OUT,
    vocabularyModule,
  ],
  [
    OUT_ACTIONS,
    actionsModule,
  ],
  [
    OUT_RUST_VOCABULARY,
    vocabularyRust,
  ],
  [
    OUT_RUST_ACTIONS,
    actionsRust,
  ],
];

// `--emit` writes nothing and prints `<repo-relative path>\0<text>\0` for each
// output instead. NUL rather than a line or a length: a generated file holds
// newlines, quotes and backslashes, and holds no NUL, so the gate splits on one
// byte and nothing has to be escaped or counted. The summary below goes to
// stderr in that mode, so stdout carries the payload and nothing else.
const emitting = process.argv.includes("--emit");
if (emitting) {
  for (const [path, text] of EMITTED) {
    process.stdout.write(`${relative(repo, path).split(sep).join("/")}\0${text}\0`);
  }
} else {
  for (const [path, text] of EMITTED) {
    mkdirSync(dirname(path), {
      recursive: true,
    });
    writeFileSync(path, text);
  }
}

const summary = emitting ? process.stderr : process.stdout;
const counted = gaps.map((gap) => `${gap.vocabulary}.${gap.variant} (${gap.missing.join(", ")})`);
// A wanted vocabulary with no rows at all names itself, because it lands in no
// `GAPS` row — there is no variant to key one on.
for (const [vocabulary, rows] of vocabularies) {
  if (rows.length === 0) counted.push(`${vocabulary} (no rows in enum-verbs.toml)`);
}
summary.write(
  `vocabulary.ts: ${[
    ...vocabularies.values(),
  ].reduce((n, rows) => n + rows.length, 0)} variants, ` + `${gaps.length} with a gap\n`,
);
for (const line of counted) summary.write(`  gap: ${line}\n`);

// The acts get their own line rather than a row in `GAPS`, because a missing
// glyph is not the same absence there. A vocabulary variant with no glyph is a
// registry that has not decided; an act with none has either that or a decision
// that it draws none, and the registry says which. Counting the undecided ones
// is the number that should fall.
const undecided = acts.filter((act) => act.absent === "undecided");
const awaiting = acts.filter((act) => act.issue !== null);
summary.write(
  `actions.ts: ${acts.length} acts, ${undecided.length} with an undecided glyph, ` +
    `${awaiting.length} not built\n`,
);
for (const act of undecided) summary.write(`  no glyph: ${act.id}\n`);
for (const act of awaiting) summary.write(`  not built: ${act.id} (#${act.issue})\n`);
