# Protocol 13.2: the limits a person changes

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`get_limits` and `save_limits` are new routes, so the minor moves. `cpu` leaves
`admission_hold` in the same change, and that is minor too: Bridge reads the set
as opaque, and a Bridge built before this simply never sees the word again.

**A value out of range does not decode.** `SaveLimits` holds each field as a
bounded integer, so the refusal is the ordinary undecodable 400 and Fleet is
never asked. Bridge bounds the field the same way, so a person meets the range
before the wire does.
