//! Where a line breaks, and where it never does.

use super::rewrap;

const PAD: &str = " This sentence is here so the line is long enough to be looked at, and it keeps on going for a while, past every limit the script would ever be given, and then some more words after that to be sure.";

fn lines(text: &str) -> Vec<String> {
    rewrap(text).split('\n').map(str::to_string).collect()
}

#[test]
fn a_long_paragraph_line_breaks_at_each_sentence() {
    let line = format!("First sentence ends here.{PAD} Last one.");
    assert_eq!(lines(&line), ["First sentence ends here.", PAD.trim(), "Last one."]);
}

#[test]
fn a_short_line_is_left_alone() {
    assert_eq!(rewrap("One. Two."), "One. Two.");
}

#[test]
fn no_break_inside_code_links_or_emphasis() {
    let line = format!("See `a. B` and [x. Y](z.md) and **bold. Still bold** here.{PAD}");
    assert_eq!(lines(&line), ["See `a. B` and [x. Y](z.md) and **bold. Still bold** here.", PAD.trim()]);
}

#[test]
fn an_abbreviation_an_initial_or_a_number_is_not_a_sentence_end() {
    for stop in ["e.g.", "No.", "J.", "step 4."] {
        let line = format!("Use {stop} This and that{PAD}");
        assert_eq!(lines(&line).len(), 1, "{stop}");
    }
}

#[test]
fn a_list_item_continues_indented_and_a_quote_keeps_its_marker() {
    assert_eq!(lines(&format!("- One.{PAD}"))[1], format!("  {}", PAD.trim()));
    assert_eq!(lines(&format!("12. One.{PAD}"))[1], format!("    {}", PAD.trim()));
    assert_eq!(lines(&format!("> One.{PAD}"))[1], format!("> {}", PAD.trim()));
}

#[test]
fn tables_headings_and_fences_are_left_alone() {
    for line in [format!("| a. B |{PAD} |"), format!("## A. B{PAD}")] {
        assert_eq!(rewrap(&line), line);
    }
    let fenced = format!("```\nOne.{PAD}\n```");
    assert_eq!(rewrap(&fenced), fenced);
}
