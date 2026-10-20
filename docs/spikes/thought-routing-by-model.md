# Spike: Can the local model route a thought, so Claude is asked only when no Session is a clear match?

**Answer: yes, for the thoughts that name a Session's topic. On 19 real reads, 3 routed at once, 16 went to
Claude, and none of the 3 disagreed with Claude. It is in force.** The three took about 70 ms
instead of 0.9 to 4.4 seconds. The other 16 are asked of Claude exactly as before, so a miss costs
nothing and a wrong direct route is the only cost; the thresholds are set so that none occurred on
these reads, with the margin shown below.

Measured on 20 Oct 2026. The code is `crates/fleet/src/helm/routing/matching.rs`; what it feeds is
`docs/concepts/helm.md`, *Routing a thought*. The model is the one `reply-asks-classifier.md`
chose, used here as a plain sentence embedder with no fine-tuned head.

## The reads

19 reads the thought router made between 12 and 17 Oct, each with the exact Sessions it was shown
and the line Claude answered, from the router's own Claude Code transcripts. The text stays
outside the repository. Another 11 reads that were written to exercise the router and 7 rows from
the store that Helm sent are not scored.

| Claude read | Reads |
|---|---|
| A thought, naming one Session | 6 |
| A thought, naming none | 7 |
| A question | 6 |

The store has no record of the person accepting or overriding a card. Where the same words later
appear as a message somewhere Claude's reading did not point, that is inferred, and it is inferred
for six of the 19. **So the measure is agreement with Claude's reading, not with the owner's
intent**, and one of the three direct routes (read 17 below) is a thought whose words went to Helm
anyway.

## What is embedded

The thought, and each candidate Session's **title**. Claude is shown the title, the repository and
up to six ledger lines; the ledger was tried and dropped.

| Candidate text | Cold embed of the Sessions offered, median | Reads routed by the thresholds that route no wrong one |
|---|---|---|
| Title | 47 ms (13 to 82) | 3 |
| Title and ledger lines | 805 ms (59 to 1,190) | 2 |

Ledger lines are file names, branch names and pull request titles of up to 120 characters each,
and the model's cost grows with the longest text in the batch. They did not separate the reads
better. Prefixing the thought with the model's retrieval instruction was tried and scored the
matches lower.

## The rule

**Route directly when the best Session scores at least 0.78 and leads the second by at least
0.10**, scores being cosine similarity of unit vectors. The thresholds are the conservative ones:

| Threshold | Why this value |
|---|---|
| 0.78 | Above the best score of any read Claude named no Session for (0.74), and below the lowest of the three routed (0.79) |
| 0.10 | Twice the lead (0.047) of the one read whose best-scoring Session was not the one Claude named |

That read is the reason the lead matters. A thought that names a Job by its number
scored 0.82 against a Session whose title repeats the number and 0.77 against the Session Claude
chose, whose branch carries it. Score alone would have routed it wrongly; the lead refused.

Three guards come before the score, none of them measured to matter on these 19:

- **A message ending in a question mark is not read here.** The model tells what a message is
  about, not whether it asks, so a question about a Session's topic would route as a thought.
- **A message of more than 300 characters is not read here.** It is a paste or several thoughts,
  and a vector of its start says little of where all of it belongs.
- **One Session has no second to lead**, so it is matched on its score alone.

## Agreement with Claude

| Read | Claude said | Best score | Lead | Result |
|---|---|---|---|---|
| 5 | Names a Session | 0.86 | 0.29 | **Routed, agrees** |
| 6 | Names a Session | 0.79 | 0.11 | **Routed, agrees** |
| 17 | Names a Session | 0.82 | 0.12 | **Routed, agrees** |
| 7 | Names a Session | 0.82 | 0.05 | Claude: best was a different Session |
| 8 | Names a Session | 0.79 | 0.09 | Claude: lead too small |
| 14 | Names a Session | 0.59 | one Session | Claude: below 0.78 |
| 1, 2 | Names none | 0.74 | 0.01 | Claude |
| 3 | Names none | 0.70 | 0.02 | Claude |
| 4 | Names none | 0.74 | 0.04 | Claude |
| 10 | Names none | 0.59 | 0.02 | Claude |
| 15 | Names none | 0.73 | 0.09 | Claude |
| 18 | Names none | 0.65 | 0.03 | Claude |
| 0, 9, 11 | A question | 0.58 to 0.63 | 0.00 to 0.01, or one Session | Claude, and each ends in a question mark |
| 12, 13, 16 | A question | 0.53 to 0.62 | 0.05 or one Session | Claude |

**Routed at once: 3 of 19. Went to Claude: 16. Disagreements: 0.** Across the thresholds, no
direct route disagreed for any score from 0.74 to 0.85 with a lead of 0.05 or more; with no lead
required, one did at scores up to 0.80 (read 7), which is why the lead is a threshold and not a tiebreak.

Nineteen reads, six of which name a Session, is little to set a threshold on. The three that routed
are ones where the thought repeats the Session's title, and the rule is not expected to catch a
thought that only describes the work. It does not need to: that is Claude's.

## Time

| | Time | Source |
|---|---|---|
| Claude, the call alone | Median 2.1 s, 0.85 to 4.4 s | First to last line of each of the 19 router transcripts. Excludes the CLI starting |
| Claude, as Fleet waits for it | 5.0 to 7.5 s | `routing.rs`, measured on a live Fleet in October, most of it the CLI starting |
| Local, thought alone | Median 22 ms, 16 to 128 ms | Release build, this machine |
| Local, Sessions not yet seen | Median 47 ms for 6 to 12 titles | The same |
| Local load | 21 ms | Weights already on disk |

A read that routes costs the thought's 22 ms once the Sessions are cached, and 70 ms on the first
read after a Session appears. A read that does not route has paid that and then pays Claude's time
as it did.

## What is in force

The reader is in force, so Fleet fetches and loads the model (133 MB) after start, which it did
not while no reader used it. Until it is loaded, and for any read it fails, Claude is asked as
before. Nothing the person sees changes except that a card appears sooner.
