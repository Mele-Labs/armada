---
name: reading-a-brief
description: Work from a brief someone handed you — find your place by search and line ranges rather than reading whole files, stay inside the paths it names, commit and push after each passing piece, and ask one QUESTION line for a decision nobody made instead of making it. Load at the start of any dispatched task.
---

# Reading a brief

**The brief says what to do; the repository says how.** Read the repository's
`CLAUDE.md` or `AGENTS.md` before the first edit. Its rules beat this file and
the brief's assumptions about it.

## Find your place, do not read everything

- Search first: grep for the names, paths and messages the brief gives.
- Read by line range around what the search found. Never read a file over 300
  lines whole.
- Check every path and name the brief cites against the tree as it is now. A
  brief can be out of date; say where it was wrong.

## Stay in the named paths

- Work in the worktree or branch the brief names. Never edit a default branch's
  checkout.
- Edit only what the brief's outcome needs. A problem you find next to it goes
  in your report, not in the diff.

## Commit and push as you go

- Commit and push after each piece that passes its checks, so a crash or a cut
  turn loses nothing.
- Run heavy commands in the foreground and wait. Never end your turn while a
  run is in the background.

## Ask, rather than decide

A decision nobody has made is not yours. A decision the person made is not
yours to reverse, even with evidence against it. If following the brief needs
one, stop at that point and put it in your report as one `**QUESTION:**` line:
what has to be decided, and the options you see.

## Report

Bottom line first: what changed, the commits, the checks and what they read.
Then anything left undone, each with who acts on it.
