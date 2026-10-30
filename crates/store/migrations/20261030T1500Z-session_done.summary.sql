-- What the agent said the Job achieved when it called `all_done`, NULL for a done kept without one.
ALTER TABLE session_done ADD COLUMN summary TEXT;
