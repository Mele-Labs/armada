# How does an Epic Job's Plan show its waves, and the Jobs that need a person?

**Decided 2026-09-30**, in the Epic-Plan board pass.

On an Epic Job, Plan drew the wave as a graph with Blocked / Waiting sections under it, each holding answer boxes, and an "Every pass of the split" list at the bottom. The board draws a strip of waves across the top, and a Job's refusal inside its panel.

**Chosen, where a Job that needs you is answered:** both. A short **Needs you** list above the graph, one line per Job, where pressing a line opens that Job's panel. The answer (a Judge's refusal, a command ask, a review) sits at the top of the panel. The Blocked / Waiting sections go.

**Cost he took:** the list repeats what the graph's nodes already say.

**Chosen, the wave strip:** drawn, in the mock. It shows each wave across the top, and pressing a past wave shows its Jobs on the graph. It replaces "Every pass of the split". Fleet does not record which pass dispatched a Job (#1692).

**Cost he took:** the strip is mock-only until #1692 lands.

**Where it landed:** `plan/epic-wave`.
