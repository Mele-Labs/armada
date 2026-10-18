# Protocol 14.15: a line of a person's own on a Link

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`StudioNodeContent::Link` gains `said`, optional: the line a person wrote beside the address saying
why they kept it, and `edit_studio_link` is the operation that changes it afterwards. `#1378`.

**Additive on both counts.** The field is left out where there is none, which is exactly the shape
every Link written before it already has, so an older Bridge reads a Link as it always did. The new
route carries the line and never the address — a Link never stops being its address — and a blank
line clears it rather than being refused. A kind that is not a Link is refused as
`fleet.studio_not_a_link`, a code added the way every other refusal here was.
