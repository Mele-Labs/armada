# What does editing a task's model change?

**Decided 2026-09-30.**

Editing a task while reviewing a plan covers title, brief, files, done-when and model. On 22 Sep the owner had ruled that the planner picks each task's difficulty and the model follows from the Job's map, so a task never names a model nobody chose a difficulty for (#1530). Asked whether editing should change the difficulty and let the model follow, or pick the model directly, he chose **pick the model directly**.

**Chosen:** the edit form offers any model, and it overrides the Job's map for that one task.

**Cost he took, stated in the question:** it reverses the 22 Sep rule, and a task's model now lives in two places, the Job's map and the task. Fleet does not carry a task's model on the wire yet (#1657).

**Where it landed:** `plan/task-dock`. Reverses the model half of #1530's 22 Sep call.
