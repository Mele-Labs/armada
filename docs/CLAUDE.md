# docs/

Written documentation. Read `.claude/skills/armada-voice/SKILL.md` before writing here — it holds the
lexicon, the prose rules and the retired terms.

- One sentence per line. `cargo xtask verify-docs` refuses a long line it would break, and
  `--write` breaks it for you.
- A bare file path refers to v1, at `v1-final`.
- A file archived out of the tree is read at main's commit, `git show ecfffb2c1ebd:<path>`.
- `spikes/` holds measured results and the raw transcript beside each one. A
  negative result is a result and stays.
