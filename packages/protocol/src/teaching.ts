// Teaching on the wire: a person presses Teach on something in a Session or in Bridge, says what they mean, and
// Fleet reads that into a proposal they check before it is kept. `crates/ipc/src/teaching.rs`,
// `crates/ipc/src/taught_mods.rs` and `docs/practices/protocol.md`. Written by hand, field for field.
//
// The wire types are the bodies and answers of `read_teaching`, `save_teaching` and `list_word_rules`.
// `TeachingRead`, `TeachingSaved` and `WordRulesRead` are Bridge's own envelopes over them, an outcome in
// place of a body.

import type { LogbookKind, LogbookSection } from "./logbook";
import type { Outcome } from "./reads";

/** How far a teaching reaches: the Session's repository, or all of them. */
export type TeachingReach = "repository" | "everywhere";

/** What Teach was pressed on. `panel`, `row` and `title_bar` are Bridge's own interface, where no agent was in the room. */
export type TeachingSourceKind = "words" | "command" | "notice" | "task" | "check" | "pull_request" | "panel" | "row" | "title_bar";

/** The thing Teach was pressed on, as a short text and the ids that name it. */
export type TeachingSource = {
  kind: TeachingSourceKind;
  /** What it shows, in a few words. A mod made from this records it as `taught_on`. */
  text: string;
  /** A `notice` source holds the notice's word first; a `panel` source holds the panel's id. */
  ids?: string[];
};

/** `read_teaching`' body (`POST /teachings/read`): what was selected or pressed, where it came from and what the person said. */
export type ReadTeaching = {
  /** The selected words. Absent for a teaching on Bridge's interface, where `source.text` says it. */
  words?: string;
  /** The message they were selected from. */
  message?: string;
  /** What the person wrote: what the thing is and what to do. */
  said: string;
  /** The Session's repository, a root `list_repositories` names. Required for the `repository` reach. */
  repository?: string;
  reach: TeachingReach;
  /** The Session it was taught in, which a rule for one Job is for. */
  session_id?: string;
  /** Absent is selected words, as before there were others. */
  source?: TeachingSource;
};

/** What an agent is to be told from now on: an entry for the Logbook. */
export type TeachingNote = {
  section: LogbookSection;
  kind: LogbookKind;
  text: string;
};

/** Where a pattern opens in the repository it was taught in, and how that was known. */
export type ProposedAddress = {
  /** An `http` or `https` address with `{match}` where the matched words go. */
  template: string;
  /** Where it came from: `settings`, `pull_request #N` or `person`. A save keeps it as it is. */
  found: string;
};

/** A pattern Bridge links, with the address for the repository it was taught in where one is known. */
export type TeachingWords = {
  /** What the pattern finds, as a person would say it: `Jira key`. */
  name: string;
  /** A regular expression Bridge runs as a JavaScript one, held to the subset both engines read. */
  pattern: string;
  /** The selected words, which the pattern matches whole. */
  example: string;
  /**
   * The address for the repository the teaching is in, where the person gave one or Fleet found one: a Words mod that already
   * has it, the Manifest's tracker, a recent merged pull request. `save_teaching` writes the one it carries. Absent where none is
   * known; a save then looks in the background and asks the person once if it finds none.
   */
  address?: ProposedAddress;
};

/** What Fleet does with a notice. */
export type FleetDoes = "tell" | "quiet" | "batch" | "call";

/**
 * What Fleet can test about a Job when a notice is sent. A closed set: a rule asks about nothing else. Each is read from
 * what Fleet already keeps, never from the forge at the moment of sending, and a fact Fleet does not know does not hold.
 * `blocks_merge`: the Job has an open pull request that cannot merge now (it conflicts, fails a required check, or is
 * unmergeable in the merge queue). `open_pull_request`: it has an open one, a draft included. `working`: its agent is
 * mid-turn. `owner_away`: sleep mode is on.
 */
export type FleetCondition = "blocks_merge" | "open_pull_request" | "working" | "owner_away";

/** A rule's other answer: when `if` holds, Fleet does `does` in place of the rule's own. */
export type FleetInstead = {
  if: FleetCondition;
  does: FleetDoes;
};

/**
 * One notice kind, what Fleet does with it, and where. It covers a notice of its word whose text matches `matches`, when it
 * has one. When the notice is sent, the first of `when` that holds replaces `does`; with none holding, `does` stands.
 */
export type FleetRule = {
  /** A notice's word: what a thread row's `notice` carries. */
  notice: string;
  does: FleetDoes;
  /** One string: `job <id>`, `repo <manifest>` or `everywhere`. */
  where: string;
  /**
   * A pattern over the notice's text, a JavaScript-safe regular expression held to a Words pattern's rules except that it
   * finds its words anywhere in the text. Absent, every notice of the word.
   */
  matches?: string;
  /** What to do instead, in the order tried. Absent when there is none. */
  when?: FleetInstead[];
};

/**
 * The Fleet part of a proposal: the rule, and what the person said that Fleet cannot test. Only the rule is ever kept in a
 * file; a rule saved with `untestable` is saved without it.
 */
export type ProposedFleet = FleetRule & {
  /** The person's own words for each condition they named that is not a `FleetCondition`. Fleet invents none in its place. */
  untestable?: string[];
};

/** One token a theme tweak sets. */
export type TweakChange = {
  /** A design token, `--bg-raised`. */
  token: string;
  /** The value the shipped theme gives it. Fleet fills this in. */
  was: string;
  /** The value to set. */
  now: string;
  /** The tokens that are aliases of this one, so they move with it. */
  used_in?: string[];
  /** The token's description in the tokens file, where it has one. */
  note?: string;
};

/** A layout change, written as the `layout.json` text a layout mod holds. */
export type TeachingLayout = {
  /** What the mod is called, which Fleet makes a name of. */
  name: string;
  layout: string;
};

/** `read_teaching`' answer: at least one part is present. Nothing is kept. */
export type TeachingProposal = {
  note?: TeachingNote;
  words?: TeachingWords;
  fleet?: ProposedFleet;
  tweak?: TweakChange[];
  layout?: TeachingLayout;
};

/**
 * `save_teaching`' body (`POST /teachings`): the proposal as the person left it, flat, and where it was taught. Every part
 * is checked again as `read_teaching` checks it; a tweak's `was`, `used_in` and `note` are not read.
 */
export type SaveTeaching = TeachingProposal & {
  reach: TeachingReach;
  /** A root `list_repositories` names. Required for the `repository` reach and for a `repo` note. */
  repository?: string;
  /** The Session it was taught in, which is told the note and cited as the entry's evidence. */
  session_id?: string;
  /** Where in that Session's record it was taught: a message, a notice or a Check. */
  cite?: string;
  /** What Teach was pressed on, which each mod records as `taught_on`. */
  source?: TeachingSource;
};

/** `save_teaching`' answer. */
export type SavedTeaching = {
  /** The Logbook entry made from the note. */
  entry?: string;
  /** The mods written or added to, by name. Absent when none was. */
  mods?: string[];
};

/** One pattern Bridge links, with where it opens for the repository asked about. */
export type WordRule = {
  /** The mod's name. */
  mod_name: string;
  name: string;
  pattern: string;
  example: string;
  /** An address with `{match}`, the repository's own or the one for every repository. */
  address: string;
  /** Where that address came from: `settings`, `pull_request #N` or `person`. */
  found: string;
};

/** `list_word_rules`' answer (`GET /teachings/words?repository=&session_id=`): the patterns that have an address for the repository asked for. */
export type WordRules = { rules: WordRule[] };

/** Bridge's envelope over `read_teaching`: the proposal, or why it could not be read. */
export type TeachingRead = { ok: true; value: TeachingProposal } | { ok: false; outcome: Outcome };

/** Bridge's envelope over `save_teaching`: what was kept, or the refusal. */
export type TeachingSaved = { ok: true; value: SavedTeaching } | { ok: false; outcome: Outcome };

/** Bridge's envelope over `list_word_rules`: the rules, or why they could not be read. */
export type WordRulesRead = { ok: true; value: WordRules } | { ok: false; outcome: Outcome };
