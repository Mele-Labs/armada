# A mod that is out of date

**Part of:** [Job](../session.md).

A mod loaded before the plugin was updated keeps running the old code until the person runs `/reload-plugins` in that job.
**The mod reports its version** in `started` and in the first `tuned` (`MOD_VERSION` in `plugins/armada/hooks/facts.ts`, held equal to `version` in `plugins/armada/.claude-plugin/plugin.json` by a Fleet test, so bump both together), and Fleet keeps it on the job's row.
`SessionRecord.mod_out_of_date` is true for a terminal job whose reported version is older than the `version` in the same file in the repository it stands in, or that reported none.
Bridge marks it on the row and in the header with "Mod out of date: run /reload-plugins".
Fleet reads the repository's file when it builds the record, so the mark moves with the next fact the job reports and not when the file changes.
