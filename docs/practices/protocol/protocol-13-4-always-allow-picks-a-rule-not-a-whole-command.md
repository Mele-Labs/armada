# Protocol 13.4: Always allow picks a rule, not a whole command

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#834`. `CommandInFlight` and `Refusal` each gain `rules` (the leading cuts of
the command, shortest first) and `suggested_rule` (the one pre-selected), and
`AnswerCommand` gains `rule` — the one a person picked, read only where the
answer is Always allow. All three are additive: an old Bridge neither reads
`rules` nor sends `rule`, so it keeps writing the whole command as the rule,
exactly as it always has, and the minor moves rather than the major.
