-- Which of its Job's terminals a terminal watch waits on, as the ID `t2`, resolved when the watch
-- starts. Absent for a watch that runs a command, and for one started before this column, which
-- listens to the Job's lowest-numbered open terminal. Additive.
ALTER TABLE watches ADD COLUMN terminal TEXT;
