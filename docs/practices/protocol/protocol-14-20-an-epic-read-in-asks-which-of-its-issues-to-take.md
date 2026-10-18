# Protocol 14.20: an Epic read-in asks which of its issues to take

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1405`. `ReadInLink` gains `take`, `everything` or `open`, and `EpicRead` gains `took`, `left_out`,
`kept` and `laid_out_from`. All additive, and `take` is asked on an Epic alone: every other kind has
one thing to read and nothing to ask about.

**Reading an Epic in again with the other answer widens or narrows what is on the Studio.** Fleet
makes the Issue nodes the answer wants and are not there, and takes back the ones it made that the
answer no longer wants — except any a person has since worked on, which the Epic counts as `kept`.

**An older peer is read as taking everything**, which is what reading one in used to do, so a Fleet
ahead of Bridge sends `took` and a Bridge behind it ignores it. An Epic read in before this version
carries no `took`, and is drawn saying nothing about an answer nobody gave it rather than claiming
one — which is the same rule `title` and `state` already follow at 14.18.

`laid_out_from` is Fleet's own bookkeeping on the wire: the corner of the block an Epic's issues sit
in, so a widening fills that block's gaps and a node dragged out of it is readable as dragged.
