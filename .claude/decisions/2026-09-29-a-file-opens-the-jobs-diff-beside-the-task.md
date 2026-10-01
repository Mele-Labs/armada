# What does pressing a file in the task panel show?

**Decided 2026-09-29.**

The owner asked whether a file in the task panel could open its diff in a second panel to the left. Fleet serves the Job's whole patch and nothing per task.

**Chosen:** a second panel opens to the left of the task panel, showing everything the Job changed in that file.

**Cost he took:** where two tasks wrote one file, both tasks' edits show together.

**Where it landed:** `plan/task-dock`.
