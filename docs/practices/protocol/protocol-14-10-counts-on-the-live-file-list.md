# Protocol 14.10: counts on the live file list

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1187`. `ChangedFile.lines`, additive and left out where absent: what a file
gained and lost, on `job.files_changed` only. Counting is the walk that renders
the patch, so Fleet counts on a due reading only once the Drone has made no call
since the reading before, something moved since the last count, and ten seconds
have passed since it. A reading between two counts carries the last count for
each file still listed, and none for a file that arrived since. Absent is not
zero, as on `TouchedFile.lines`.
