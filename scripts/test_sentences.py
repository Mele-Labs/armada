"""`scripts/sentences` breaks a long prose line at sentence ends and touches nothing else.

python3 scripts/test_sentences.py
"""
import importlib.machinery
import importlib.util
import pathlib
import unittest

HERE = pathlib.Path(__file__).resolve().parent
loader = importlib.machinery.SourceFileLoader("sentences", str(HERE / "sentences"))
spec = importlib.util.spec_from_loader("sentences", loader)
sentences = importlib.util.module_from_spec(spec)
loader.exec_module(sentences)

PAD = " This sentence is here so the line is long enough to be looked at, and it keeps on going for a while, past every limit the script would ever be given, and then some more words after that to be sure."


class Sentences(unittest.TestCase):
    def test_a_long_paragraph_line_breaks_at_each_sentence(self):
        line = "First sentence ends here." + PAD + " Last one."
        self.assertEqual(sentences.rewrap(line).split("\n"), ["First sentence ends here.", PAD.strip(), "Last one."])

    def test_a_short_line_is_left_alone(self):
        self.assertEqual(sentences.rewrap("One. Two."), "One. Two.")

    def test_no_break_inside_code_links_or_emphasis(self):
        line = "See `a. B` and [x. Y](z.md) and **bold. Still bold** here." + PAD
        self.assertEqual(sentences.rewrap(line).split("\n"), ["See `a. B` and [x. Y](z.md) and **bold. Still bold** here.", PAD.strip()])

    def test_an_abbreviation_is_not_a_sentence_end(self):
        line = "Use e.g. This and that" + PAD
        self.assertTrue(sentences.rewrap(line).startswith("Use e.g. This and that This sentence"))

    def test_a_list_item_continues_indented_and_a_quote_keeps_its_marker(self):
        self.assertEqual(sentences.rewrap("- One." + PAD).split("\n")[1], "  " + PAD.strip())
        self.assertEqual(sentences.rewrap("> One." + PAD).split("\n")[1], "> " + PAD.strip())

    def test_tables_headings_and_fences_are_left_alone(self):
        for line in ["| a. B |" + PAD + " |", "## A. B" + PAD]:
            self.assertEqual(sentences.rewrap(line), line)
        fenced = "```\nOne." + PAD + "\n```"
        self.assertEqual(sentences.rewrap(fenced), fenced)


if __name__ == "__main__":
    unittest.main()
