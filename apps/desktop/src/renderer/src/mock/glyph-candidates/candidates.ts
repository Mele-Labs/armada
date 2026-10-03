// Glyph candidates for five meanings the icon registry has no glyph for, each
// with the registry entry it would land as. SCRATCH: the owner picks one per
// meaning in the `glyphCandidates` walk, the chosen entries move into
// `packages/icons/icons.toml`, and this directory is deleted with that change.
//
// Every glyph here is imported by name, so rule seventeen (`xtask/src/
// rules_icons.rs`) names each as unregistered while this branch stands. That is
// the true state of a candidate, and the branch does not land.

import {
  Asterisk,
  Diff,
  Ellipsis,
  GitCompare,
  Hash,
  Loader,
  PenLine,
  Quote,
  Signature,
  SquareDot,
  TextCursor,
  TextCursorInput,
  TextQuote,
  Ticket,
  UserPen,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

/** One candidate, as the `[icons.<name>]` table it would become. */
export type Candidate = {
  /** The lucide-react name, which is the registry key. */
  name: string;
  glyph: LucideIcon;
  means: string;
  group: string;
  size: "12px";
  status: "Proposed";
  reserved: string;
  notes: string;
  /** How it moves, for a settling mark. */
  motion?: "blink" | "sequence" | "turn";
};

export type Meaning = {
  id: "issue" | "prompt" | "person" | "moved" | "settling";
  /** What the glyph has to say, as the brief put it. */
  says: string;
  /** Where it draws. */
  where: "criterion" | "moved" | "row";
  /** The candidate recommended, by name. Said in the report, never on the sheet. */
  recommended: string;
  candidates: readonly Candidate[];
};

const ORIGIN = "Criterion origin";

export const MEANINGS: readonly Meaning[] = [
  {
    id: "issue",
    says: "Origin: an issue",
    where: "criterion",
    recommended: "ticket",
    candidates: [
      {
        name: "ticket",
        glyph: Ticket,
        means: "A ticket stub, notched at both ends and perforated down its length",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words were read out of an issue, on that criterion's origin line only. Never a Job, a request or a dispatch, never a pull request, which is `git-pull-request`, and never a place in a queue.",
        notes:
          "The forge's own issue mark is a ring around a dot, which is `circle-dot`, running's by reservation, so the issue is drawn as the thing it is called rather than the forge's picture of it. A notched stub is the only outline in the set with bites out of both short edges, so it holds at 12px. Refused: `circle-dot` (running), `file-*` (evidence), `message-square` (Helm), `scroll-text` (a workflow).",
      },
      {
        name: "hash",
        glyph: Hash,
        means: "A number sign",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words were read out of an issue, leading the issue's reference on its origin line only. Never a count, a step number, a handle, a channel or a tag.",
        notes:
          "Four strokes with no interior, so it is the cleanest of the three at 12px. The cost: the reference beside it already reads `armada#1162`, so the line says # twice, and # says a number rather than an issue.",
      },
      {
        name: "square-dot",
        glyph: SquareDot,
        means: "A rounded square with a dot at its centre",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words were read out of an issue, on its origin line only. Never a checkbox, a radio, a selection or a running mark: a ring around a dot is running's, and this is that construction squared.",
        notes:
          "The nearest honest echo of the forge's issue mark that the circle-* reservation leaves free. Rule 4 permits it, because the origin line is --fg-muted and running is teal, but it shares circle-dot's filled-centre construction, and at 12px it can read as an unticked box.",
      },
    ],
  },
  {
    id: "prompt",
    says: "Origin: your request",
    where: "criterion",
    recommended: "quote",
    candidates: [
      {
        name: "quote",
        glyph: Quote,
        means: "A pair of opening quotation marks",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words came from the request a person typed at dispatch, on its origin line only. Never a citation of an issue or a document, never Helm's words, and never a Drone's or a Judge's text.",
        notes:
          "Your own words, quoted back to you. Two filled-weight marks with no interior detail, so it is the heaviest small silhouette of the three and holds at 12px. Nothing else in the set is drawn as punctuation.",
      },
      {
        name: "text-cursor-input",
        glyph: TextCursorInput,
        means: "A text field with an I-beam cursor standing in it",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words came from the request typed at dispatch, on its origin line only. Never a field, an input or an edit control, and never a field still being written.",
        notes:
          "The most literal: words typed into a box. The cost is rule 3, the cursor inside the box is interior detail at 12px, and it reads as a control. It cannot stand beside `text-cursor` if that is chosen for settling, since both carry the same I-beam.",
      },
      {
        name: "text-quote",
        glyph: TextQuote,
        means: "Ruled lines, the lower two set in behind a bar down the leading edge",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion whose words came from the request typed at dispatch, on its origin line only. Never a blockquote control, a log, a transcript or a brief.",
        notes:
          "A quoted passage. Clear of `text-wrap` by rule 4, which admits a collision across groups and sizes (16px Console against 12px here), but ruled lines are a crowded family at 12px and the bar is the whole difference.",
      },
    ],
  },
  {
    id: "person",
    says: "Origin: a person, at the gate",
    where: "criterion",
    recommended: "user-pen",
    candidates: [
      {
        name: "user-pen",
        glyph: UserPen,
        means: "A head and shoulders, with a pen across the lower right",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion a person added or reworded at the gate, on its origin line only. Never a person required, which is `user-check`, never an attestation verdict, and never rename or edit, which are `square-pen`'s.",
        notes:
          "The human-figure family is reserved to `human required, or actor=human`, and a person who wrote the line is actor=human. Against `user-check` rule 4 holds: that one is an amber 12px badge, this one is a --fg-muted origin line. The pen is detail and may blur at 12px; the figure is the silhouette, and it is what says a person.",
      },
      {
        name: "signature",
        glyph: Signature,
        means: "A scrawl over a baseline",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion a person added or reworded at the gate, on its origin line only. Never an attestation, a sign-off or an approval: `stamp` and `check` hold those.",
        notes:
          "In the person's own hand, with no figure, so it keeps clear of the human-figure reservation. The risk is meaning: a signature reads as signing off, which is the open `[attested-verdict-glyph]` question's ground, and a criterion row is where that verdict would draw.",
      },
      {
        name: "pen-line",
        glyph: PenLine,
        means: "A pen over a short rule",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "A criterion a person added or reworded at the gate, on its origin line only. Never edit, rename, compose or drawing: `square-pen`, `type` and `pencil` hold those.",
        notes:
          "Somebody wrote this line. The weakest separation of the three: at 12px it is `pencil` with a rule under it, and `pencil`'s own reservation exists to stop a pen meaning a person writing in a field.",
      },
    ],
  },
  {
    id: "moved",
    says: "The issue has moved since",
    where: "moved",
    recommended: "diff",
    candidates: [
      {
        name: "diff",
        glyph: Diff,
        means: "A plus over a rule: what was, and something more",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "The issue a criterion was read from has been edited since Fleet read it, leading that note only. Never a change count, a diff view or a produced file's change: `file-diff` holds that, and a diff's gutter draws `+` and `-` as signs.",
        notes:
          "Says *differs* in three strokes. It inherits the note's caution hue, which is the note's and not the glyph's. The risk: on its own it reads as add, and the canvas rail's `+` is a sign in a square, so it should never sit on a canvas.",
      },
      {
        name: "git-compare",
        glyph: GitCompare,
        means: "Two nodes with arrows running between them: two versions, compared",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "The issue a criterion was read from has been edited since Fleet read it, leading that note only. Never a branch comparison, a merge or a diff view.",
        notes:
          "The most literal: what was read against what is there now. The cost is rule 3, two small rings and two arrowheads are interior detail at 12px. It also borrows the git family's look for something that is not git.",
      },
      {
        name: "asterisk",
        glyph: Asterisk,
        means: "A six-armed asterisk",
        group: ORIGIN,
        size: "12px",
        status: "Proposed",
        reserved:
          "The issue a criterion was read from has been edited since Fleet read it, leading that note only. Never a required field, a footnote, a wildcard or an unsaved-change mark.",
        notes:
          "Three strokes crossing, so it is the sharpest at 12px, and it reads as *see the note*, which is what the note is. It says nothing about change on its own; the sentence beside it carries all of that.",
      },
    ],
  },
  {
    id: "settling",
    says: "A field the proposer is still settling",
    where: "row",
    recommended: "text-cursor",
    candidates: [
      {
        name: "text-cursor",
        glyph: TextCursor,
        means: "An I-beam text cursor",
        group: "Proposing",
        size: "12px",
        status: "Proposed",
        motion: "blink",
        reserved:
          "A field on a proposing Job's row or page that the proposer has not written yet, blinking. Never a text field, a rename or an edit, never a Job status, which is `scan-line`, and never a loading or busy mark anywhere else.",
        notes:
          "The proposer is writing the answer, and a blinking caret is what writing looks like. The I-beam is the only outline in the set that is a cursor, so it collides with nothing. It blinks at --duration-pulse and holds still under reduced motion, where the empty cell still reads as unsettled.",
      },
      {
        name: "ellipsis",
        glyph: Ellipsis,
        means: "Three dots in a row",
        group: "Proposing",
        size: "12px",
        status: "Proposed",
        motion: "sequence",
        reserved:
          "A field on a proposing Job's row or page that the proposer has not written yet, its dots brightening in turn. Never a more-actions menu, an overflow or a truncation.",
        notes:
          "The typing indicator, three dots stepping in turn. The cost is convention: a still ellipsis is a more-actions button almost everywhere else, and under reduced motion that is all that is left of it.",
      },
      {
        name: "loader",
        glyph: Loader,
        means: "Eight spokes around an empty centre",
        group: "Proposing",
        size: "12px",
        status: "Proposed",
        motion: "turn",
        reserved:
          "A field on a proposing Job's row or page that the proposer has not written yet, turning. Never a page or a read waiting on Fleet, a fetch, or a generic busy state.",
        notes:
          "Unmistakably busy. That is also its cost: it is the generic spinner, the meaning `scan-line` and `circle-dashed` are each kept from, and Bridge stands in for data that has not arrived with a Skeleton rather than a spinner.",
      },
    ],
  },
];
