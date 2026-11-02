#!/usr/bin/env python3
#
# `scripts/format-catch-up` against a throwaway repository and a stub `pnpm`
# whose `format` splits `key: a, b, c` onto one item per line, the way the
# formatter breaks a long literal.
#
#   python3 scripts/test_format_catch_up.py

import os
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CATCH_UP = os.path.join(HERE, "scripts", "format-catch-up")

STUB_PNPM = """#!/usr/bin/env python3
import subprocess, sys
if sys.argv[1:2] != ["format"]:
    sys.exit(0)
for path in subprocess.run(["git", "ls-files", "*.ts"], capture_output=True, text=True).stdout.split():
    lines = []
    for line in open(path).read().splitlines():
        key, sep, items = line.partition(": ")
        if sep and ", " in items:
            lines.append(key + ":")
            parts = items.split(", ")
            lines += ["  " + p + ("," if i < len(parts) - 1 else "") for i, p in enumerate(parts)]
        else:
            lines.append(line)
    open(path, "w").write("\\n".join(lines) + "\\n")
"""


class FormatCatchUp(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = os.path.join(self.tmp, "repo")
        bin_dir = os.path.join(self.tmp, "bin")
        os.makedirs(bin_dir)
        pnpm = os.path.join(bin_dir, "pnpm")
        with open(pnpm, "w") as f:
            f.write(STUB_PNPM)
        os.chmod(pnpm, 0o755)
        self.env = {**os.environ, "PATH": bin_dir + os.pathsep + os.environ["PATH"],
                    "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t",
                    "GIT_COMMITTER_EMAIL": "t@t", "GIT_CONFIG_GLOBAL": os.devnull}
        self.git("init", "-q", "-b", "main", self.repo, cwd=self.tmp)
        self.write("a.ts", "header\nitems: alpha, beta, gamma\nfooter\n")
        self.write("b.ts", "other: one, two\n")
        self.commit("base")
        self.git("branch", "feature")

    def git(self, *args, cwd=None):
        return subprocess.run(["git", *args], cwd=cwd or self.repo, env=self.env,
                              capture_output=True, text=True)

    def write(self, path, text):
        with open(os.path.join(self.repo, path), "w") as f:
            f.write(text)

    def read(self, path):
        with open(os.path.join(self.repo, path)) as f:
            return f.read()

    def commit(self, message):
        self.git("add", "-A")
        self.assertEqual(self.git("commit", "-q", "-m", message).returncode, 0)
        return self.git("rev-parse", "HEAD").stdout.strip()

    def land_the_reformat(self):
        """main gains the config commit, the reformat and the blame list, then one more change."""
        self.git("checkout", "-q", "main")
        self.write("biome.jsonc", "{}\n")
        self.commit("config")
        subprocess.run(["pnpm", "format"], cwd=self.repo, env=self.env, check=True)
        reformat = self.commit("reformat")
        self.write(".git-blame-ignore-revs", "# The reformat\n" + reformat + "\n")
        self.commit("blame")
        self.write("c.ts", "later: x, y\n")
        self.commit("later")
        return reformat

    def catch_up(self):
        return subprocess.run([CATCH_UP, "main"], cwd=self.repo, env=self.env,
                              capture_output=True, text=True)

    def test_a_plain_merge_of_the_reformat_conflicts(self):
        self.git("checkout", "-q", "feature")
        self.write("a.ts", "header\nitems: alpha, beta, gamma, delta\nfooter\n")
        self.commit("delta")
        self.land_the_reformat()
        self.git("checkout", "-q", "feature")
        self.assertNotEqual(self.git("merge", "--no-edit", "main").returncode, 0)

    def test_the_branch_catches_up_with_its_change_formatted(self):
        self.git("checkout", "-q", "feature")
        self.write("a.ts", "header\nitems: alpha, beta, gamma, delta\nfooter\n")
        self.commit("delta")
        self.land_the_reformat()
        self.git("checkout", "-q", "feature")
        done = self.catch_up()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.read("a.ts"), "header\nitems:\n  alpha,\n  beta,\n  gamma,\n  delta\nfooter\n")
        self.assertEqual(self.read("b.ts"), "other:\n  one,\n  two\n")
        self.assertEqual(self.read("c.ts"), "later:\n  x,\n  y\n")
        self.assertEqual(self.git("merge-base", "--is-ancestor", "main", "HEAD").returncode, 0)
        self.assertEqual(self.git("status", "--porcelain").stdout, "")

    def test_a_real_conflict_stops_before_the_reformat(self):
        self.git("checkout", "-q", "feature")
        self.write("a.ts", "header\nitems: alpha, beta, gamma, delta\nfooter\n")
        self.commit("delta")
        self.git("checkout", "-q", "main")
        self.write("a.ts", "header\nitems: alpha, beta, gamma, epsilon\nfooter\n")
        self.commit("epsilon")
        reformat = self.land_the_reformat()
        self.git("checkout", "-q", "feature")
        done = self.catch_up()
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("resolve", done.stderr)
        self.assertNotEqual(self.git("merge-base", "--is-ancestor", reformat, "HEAD").returncode, 0)

    def test_a_branch_already_past_the_reformat_only_merges_main(self):
        self.land_the_reformat()
        self.git("checkout", "-q", "feature")
        self.assertEqual(self.catch_up().returncode, 0)
        self.write("b.ts", "other:\n  one,\n  two,\n  three\n")
        self.commit("three")
        self.git("checkout", "-q", "main")
        self.write("d.ts", "more: p, q\n")
        self.commit("more")
        self.git("checkout", "-q", "feature")
        done = self.catch_up()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.read("b.ts"), "other:\n  one,\n  two,\n  three\n")
        self.assertEqual(self.read("d.ts"), "more:\n  p,\n  q\n")

    def test_an_unclean_tree_is_refused(self):
        self.land_the_reformat()
        self.git("checkout", "-q", "feature")
        self.write("a.ts", "edited\n")
        done = self.catch_up()
        self.assertNotEqual(done.returncode, 0)
        self.assertEqual(self.read("a.ts"), "edited\n")


if __name__ == "__main__":
    unittest.main()
