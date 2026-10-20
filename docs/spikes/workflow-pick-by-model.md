# Spike: Can the local model pick a new Session's workflow, model and effort, so Claude is asked only when it is not sure?

**Answer: no. On 38 real requests no threshold picks a workflow without disagreeing with Claude, so
none was picked locally. Model and effort are not learnable from this data (the best rule agreed
with Claude on 23 of 38). The reader exists and is not in force.** Even a perfect workflow pick
would skip no call: the one call that proposes the workflow also gives the Session its title, model
and effort.

Measured on 20 Oct 2026. The code is `crates/fleet/src/session_host/workflow_pick.rs`; what it would
feed is `docs/concepts/session/a-jobs-proposal.md`. The model is the one `reply-asks-classifier.md`
chose, used as a plain sentence embedder with no fine-tuned head, as `thought-routing-by-model.md`
did. The request text stays outside the repository.

## The data

38 requests the Session proposer read between 16 and 20 Oct, each with the workflows the repository
held then (7 or 8; none in one), Claude's proposal and what the Session started on, from the
proposer's own Claude Code transcripts and the store.

| | Count |
|---|---|
| Requests | 38 |
| Claude proposed `none` | 6 |
| Claude proposed a held workflow | 32 (feature 18, bug 12, design plan 2) |
| Held no workflow at all | 1 (and so `none`) |
| Workflow the person changed | 0 |
| Model the person changed | 2 (proposed opus, started sonnet; proposed sonnet, started opus) |
| Proposed model and effort | opus/high 25, sonnet/medium 9, haiku/low 3, opus/medium 1 |

**So the measure is agreement with Claude's proposal.** Where the Session started on something else
(the model, twice), Claude's proposal and the start differ, and the rules below are scored against
both.

## Workflow: embed the request and each workflow

The request and each held workflow's text, cosine similarity of unit vectors. Three forms of the
workflow's text were tried: `for` alone, `name: for`, and the name alone.

| Workflow text | Top workflow is Claude's | Highest top score when right | Highest top score when wrong |
|---|---|---|---|
| `for` alone | 0 of 37 | none | 0.71 |
| `name: for` | 15 of 37 | 0.64 | 0.66 |
| name alone | 7 of 37 | 0.63 | 0.65 |

A best score between 0.52 and 0.71 on every request, with the best leading the second by 0.00 to 0.08.
The `for` text describes the work in the words of a definition ("Undoing a change that already
landed") and a request is written as a want ("the importer drops the last line"), so they are not
near each other. The `for` text alone put `revert` first for 23 requests.

A search over every threshold from 0.50 to 0.89 and every lead from 0.00 to 0.29 found none that
picks any request with no disagreement, for any of the three forms. **Chosen, since a rule that
picks nothing is the only safe one: `name: for`, a pick at 0.70 or above leading the second by
0.10 or more.** The highest best score on the 38 was 0.66 and the widest lead 0.05, so it picks 0 of
37 and cannot disagree.

| | Picked locally | To Claude | Disagreements |
|---|---|---|---|
| Workflow | 0 | 38 | 0 |

`none` is never picked locally. Six requests were `none` for Claude and the model had a top
workflow for each at 0.58 to 0.66.

A nearest-neighbour vote over the other requests (leave one out) was also tried, as the data-driven
alternative: the vector of each past request against Claude's workflow for it. With one neighbour
and no score floor it was wrong on 13 of 31 picks; unanimous over three neighbours at 0.85 it picked
2 with none wrong, which is two requests and a threshold set on the same 38. Fleet does not keep a
past request's vector, so it was not built.

## Model and effort

| Rule | Agrees with Claude on model and effort |
|---|---|
| Always opus/high | 25 of 38 |
| The workflow's most common choice, leaving the request out | 23 of 38 |
| Nearest request's choice (leave one out) | 17 of 38 |
| Three nearest, where they agree (leave one out) | 11 of 17 that agreed |
| Five nearest, where they agree (leave one out) | 4 of 7 that agreed |
| The workflow's most common choice against what started | 22 of 38 |

Model and effort are not determined by the workflow: `bug` was opus/high for 7 and sonnet/medium
for 5, and `feature` was opus/high for 14 and one each of three other pairs. What decides it is how
hard the work reads, which Claude is asked and the model's vectors do not carry here. **Agreement
is nowhere near perfect, so model and effort stay with Claude.**

## Time

| | Time | Source |
|---|---|---|
| Claude, the call alone | Median 3.2 s, 0.7 to 4.8 s | First to last line of each of the 38 transcripts. Excludes the CLI starting |
| Local, the request alone | Median 100 ms; 36 ms for the 22 under 1,000 characters; 21 to 350 ms | Release build, this machine |
| Local, the request and 21 workflow texts at once | Median 1.3 s, up to 6.8 s | The same. The batch pads to its longest text, and a request of up to 12,000 characters is cut at 512 tokens |

The workflows' vectors are kept between reads, so a read after the first embeds the request only.

## Skipped calls

**None.** The workflow reader picks none of the 38. And if it did, `proposed_for_session` is the
one call that settles the Session's title, workflow, model and effort together; `reading_ended`
fills each from its answer. Picking the workflow locally leaves three fields for the call, so it
still happens. Skipping it would also change what the person sees: with no answer the title falls
back to the first words of the request (`title_of`) and not Claude's short title.

## What is in force

**Not in force.** `workflow_pick::IN_FORCE` is false, so Fleet fetches and loads nothing for it and
nothing calls it. `wanted_by` names it beside the other readers, so turning it on is one constant
and its thresholds are in the module. It would need all of: a workflow pick that disagrees with
Claude on none of a larger set, a way to pick model and effort that agrees with Claude near
perfectly, and a title that does not need the call.

Thirty-eight requests, 32 with a held workflow Claude picked, is little to set a threshold on, and the figures
here are a reason to run them again on more, not to adopt a rule.
