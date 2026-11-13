//! What a question asked in pictures works out, with nothing drawn: where a still's parts sit when it is scaled to a
//! column, which part a press lands on, the parts taken into a mix and the words each is read by, the body Send mix
//! posts, and which forms a Job's result is also offered in. `look_question.rs` draws it, `look_done.rs` draws the result.

use ipc::{AnswerWaitingParts, OptionPicture, PartPicked, PicturePart, SessionId, WaitingOption};

use crate::ways::name_of;

/// A part's box as fractions of its still, so it scales with the still wherever the still is drawn: left, top, width, height.
pub(super) fn fractions(picture: &OptionPicture, part: &PicturePart) -> [f32; 4] {
    let (w, h) = (picture.width.max(1) as f32, picture.height.max(1) as f32);
    [part.x as f32 / w, part.y as f32 / h, part.w as f32 / w, part.h as f32 / h]
}

/// The part a press at (`x`, `y`) of the still, drawn `shown` points wide, lands in: the smallest of those that hold it,
/// so a part inside another is the one pressed. `None` is a press outside every part, which takes the option whole.
pub(super) fn part_at(picture: &OptionPicture, shown: f32, x: f32, y: f32) -> Option<usize> {
    let scale = shown / picture.width.max(1) as f32;
    picture
        .parts
        .iter()
        .enumerate()
        .filter(|(_, part)| {
            let (left, top) = (part.x as f32 * scale, part.y as f32 * scale);
            x >= left && x < left + part.w as f32 * scale && y >= top && y < top + part.h as f32 * scale
        })
        .min_by_key(|(_, part)| u64::from(part.w) * u64::from(part.h))
        .map(|(at, _)| at)
}

/// What a part's label tab measures, in points as drawn: one character's width, the tab's height, how far it is lifted above its
/// part's top edge, how far in from the part's left edge, the padding either side of its text, and the gap kept between tabs.
#[derive(Clone, Copy, Debug)]
pub(super) struct Tab {
    pub char_width: f32,
    pub height: f32,
    pub lift: f32,
    pub inset: f32,
    pub pad: f32,
    pub gap: f32,
}

/// How far down each part's label tab steps from its part's top-left so that no two tabs in a picture overlap, whatever width the
/// picture is drawn at. Parts keep their own order in the answer; a tab goes down by whole steps under the tab it would have
/// covered, so it stays at its part's left edge. The tab is judged wider than its text, so a measured one never reaches further.
pub(super) fn label_drops(picture: &OptionPicture, shown: f32, tab: Tab) -> Vec<f32> {
    // A picture not yet drawn has no width to place against.
    if shown <= 0. {
        return vec![0.; picture.parts.len()];
    }
    let still_height = shown * picture.height.max(1) as f32 / picture.width.max(1) as f32;
    let rects: Vec<[f32; 3]> = picture
        .parts
        .iter()
        .map(|part| {
            let [x, y, ..] = fractions(picture, part);
            [x * shown + tab.inset, y * still_height - tab.lift, part.label.chars().count() as f32 * tab.char_width + 2. * tab.pad]
        })
        .collect();
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|a, b| (rects[*a][1], rects[*a][0]).partial_cmp(&(rects[*b][1], rects[*b][0])).unwrap_or(std::cmp::Ordering::Equal));
    let mut drops = vec![0.; rects.len()];
    let mut placed: Vec<usize> = Vec::new();
    for at in order {
        let [left, top, wide] = rects[at];
        let mut drop = 0.;
        // Each pass moves the tab under one tab it covers; the tabs already placed are finitely many, so this ends.
        while let Some(under) = placed.iter().copied().find(|&other| {
            let [other_left, other_top, other_wide] = rects[other];
            let other_top = other_top + drops[other];
            left < other_left + other_wide + tab.gap
                && other_left < left + wide + tab.gap
                && top + drop < other_top + tab.height + tab.gap
                && other_top < top + drop + tab.height + tab.gap
        }) {
            drop = rects[under][1] + drops[under] + tab.height + tab.gap - top;
        }
        drops[at] = drop;
        placed.push(at);
    }
    drops
}

/// One part of one option's picture, taken into the mix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Taken {
    pub option: u32,
    pub part: String,
}

/// Take a part, or let it go where it was taken: a second press on a taken part. Taken parts keep the order they were taken in.
pub(super) fn toggled(taken: &mut Vec<Taken>, option: u32, part: &str) {
    match taken.iter().position(|one| one.option == option && one.part == part) {
        Some(at) => {
            taken.remove(at);
        }
        None => taken.push(Taken { option, part: part.to_string() }),
    }
}

/// Whether this part is in the mix.
pub(super) fn is_taken(taken: &[Taken], option: u32, part: &str) -> bool {
    taken.iter().any(|one| one.option == option && one.part == part)
}

/// The words a taken part reads by in the mix bar: its label, then the name of the option it comes from, without the Job's mark.
pub(super) fn pill(options: &[WaitingOption], one: &Taken) -> Option<String> {
    let option = options.get(one.option as usize)?;
    let part = option.picture.as_ref()?.parts.iter().find(|part| part.name == one.part)?;
    Some(format!("{} from {}", part.label, name_of(&option.label)))
}

/// What Send mix posts to `answer_waiting_parts`: the parts taken, each an option's index and a part's name.
pub(super) fn body(session: &str, item: &str, taken: &[Taken]) -> AnswerWaitingParts {
    AnswerWaitingParts {
        session_id: SessionId::carried(session),
        item_id: item.to_string(),
        parts_picked: taken.iter().map(|one| PartPicked { option: one.option, part: one.part.clone() }).collect(),
    }
}

/// What a Job made, as far as the result's Also line cares.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Produced {
    /// The Job wrote a document.
    pub page: bool,
    /// The Job showed a picture before it said it was done.
    pub picture: bool,
    /// The Job's `all_done` carries a summary to speak.
    pub summary: bool,
}

/// The other forms a result is offered in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Form {
    Page,
    Picture,
    Listen,
}

/// A chip for each form the Job produced, and none for one it did not.
pub(super) fn also(made: Produced) -> Vec<Form> {
    [(made.page, Form::Page), (made.picture, Form::Picture), (made.summary, Form::Listen)].into_iter().filter(|(made, _)| *made).map(|(_, form)| form).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, label: &str, x: u32, y: u32, w: u32, h: u32) -> PicturePart {
        PicturePart { name: name.into(), label: label.into(), x, y, w, h }
    }

    fn page() -> OptionPicture {
        OptionPicture {
            image: "/files/stacked.png".into(),
            width: 1200,
            height: 800,
            parts: vec![part("page", "Page", 0, 0, 1200, 800), part("header", "Header", 0, 0, 1200, 110), part("cards", "Cards", 60, 150, 1080, 430)],
        }
    }

    fn options() -> Vec<WaitingOption> {
        vec![
            WaitingOption { label: "Stacked (Recommended)".into(), description: None, picture: Some(page()) },
            WaitingOption { label: "Sidebar".into(), description: None, picture: Some(OptionPicture { parts: vec![part("sidebar", "Sidebar", 0, 0, 280, 800)], ..page() }) },
        ]
    }

    /// The three pictures the `ways-look` fixture asks with.
    fn fixture() -> Vec<OptionPicture> {
        let still = |height, parts| OptionPicture { height, parts, ..page() };
        vec![
            still(800, vec![part("header", "Header", 0, 0, 1200, 110), part("cards", "Cards", 60, 150, 1080, 430), part("footer", "Footer", 0, 700, 1200, 100)]),
            still(800, vec![part("sidebar", "Sidebar", 0, 0, 280, 800), part("content", "Content", 320, 140, 840, 600)]),
            still(750, vec![part("list", "List", 0, 90, 520, 660), part("detail", "Detail", 520, 90, 680, 660)]),
        ]
    }

    /// Where the 820 window draws a tab: the 12 point label squeezed to 0.7, with the tokens' lift, inset and padding.
    fn tab_at_820() -> Tab {
        Tab { char_width: 12. * 0.7 * 0.6, height: 16. * 0.7 + 2., lift: 8., inset: 6., pad: 5., gap: 1. }
    }

    fn overlap(picture: &OptionPicture, shown: f32, tab: Tab, drops: &[f32]) -> bool {
        let rects: Vec<[f32; 4]> = picture
            .parts
            .iter()
            .zip(drops)
            .map(|(part, drop)| {
                let [x, y, ..] = fractions(picture, part);
                let top = y * shown * picture.height as f32 / picture.width as f32 - tab.lift + drop;
                [x * shown + tab.inset, top, part.label.chars().count() as f32 * tab.char_width + 2. * tab.pad, tab.height]
            })
            .collect();
        rects.iter().enumerate().any(|(i, a)| rects.iter().skip(i + 1).any(|b| a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]))
    }

    #[test]
    fn no_two_labels_in_a_picture_overlap_at_the_width_820_draws_a_card_or_any_other() {
        let tab = tab_at_820();
        // What the 820 window gives a picture is about 95 points across. Left where their parts put them, the labels of the
        // stacked and the sidebar picture cover one another there.
        let narrow = fixture();
        assert!(overlap(&narrow[0], 95., tab, &[0.; 3]));
        assert!(overlap(&narrow[1], 95., tab, &[0.; 2]));
        for picture in fixture() {
            for shown in (60..=520).step_by(5) {
                let drops = label_drops(&picture, shown as f32, tab);
                assert!(!overlap(&picture, shown as f32, tab, &drops), "labels overlap at {shown} points over {:?}", picture.parts.iter().map(|part| &part.name).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn a_label_that_clears_every_other_stays_at_its_parts_top_left() {
        let tab = tab_at_820();
        let wide = label_drops(&fixture()[2], 400., tab);
        assert_eq!(wide, vec![0., 0.]);
    }

    #[test]
    fn a_part_is_a_fraction_of_its_still_so_it_scales_with_it() {
        let [x, y, w, h] = fractions(&page(), &page().parts[2]);
        // Drawn 300 wide, the still is 200 tall and the part's box is 270 across at 15 over and 37.5 down.
        let drawn = [x * 300., y * 200., w * 300., h * 200.];
        for (got, want) in drawn.into_iter().zip([15., 37.5, 270., 107.5]) {
            assert!((got - want).abs() < 1e-3, "{got} is not {want}");
        }
    }

    #[test]
    fn a_press_lands_on_the_smallest_part_that_holds_it_at_any_width() {
        let still = page();
        // At 600 wide the still is half size: (30, 20) is in the header, which sits inside the whole-page part.
        assert_eq!(part_at(&still, 600., 30., 20.), Some(1));
        // The same place on the still, 1200 wide, is the same part.
        assert_eq!(part_at(&still, 1200., 60., 40.), Some(1));
        // Between the header and the cards only the whole-page part holds it.
        assert_eq!(part_at(&still, 600., 100., 62.), Some(0));
        // Inside the cards.
        assert_eq!(part_at(&still, 600., 300., 150.), Some(2));
        // Past the still's edge there is no part: the option is taken whole.
        assert_eq!(part_at(&still, 600., 700., 150.), None);
        let sparse = OptionPicture { parts: vec![part("header", "Header", 0, 0, 1200, 110)], ..page() };
        assert_eq!(part_at(&sparse, 240., 100., 100.), None);
        assert_eq!(part_at(&sparse, 240., 100., 10.), Some(0));
    }

    #[test]
    fn a_second_press_lets_a_taken_part_go_and_the_rest_keep_their_order() {
        let mut taken = Vec::new();
        toggled(&mut taken, 0, "header");
        toggled(&mut taken, 1, "sidebar");
        toggled(&mut taken, 0, "cards");
        assert_eq!(taken.iter().map(|one| (one.option, one.part.as_str())).collect::<Vec<_>>(), [(0, "header"), (1, "sidebar"), (0, "cards")]);
        assert!(is_taken(&taken, 1, "sidebar"));
        toggled(&mut taken, 1, "sidebar");
        assert!(!is_taken(&taken, 1, "sidebar"));
        assert_eq!(taken.len(), 2);
        // The same name in another option is another part.
        toggled(&mut taken, 1, "header");
        assert!(is_taken(&taken, 1, "header") && is_taken(&taken, 0, "header"));
    }

    #[test]
    fn a_pill_reads_the_part_from_the_option_without_the_jobs_mark() {
        let options = options();
        assert_eq!(pill(&options, &Taken { option: 0, part: "header".into() }).as_deref(), Some("Header from Stacked"));
        assert_eq!(pill(&options, &Taken { option: 1, part: "sidebar".into() }).as_deref(), Some("Sidebar from Sidebar"));
        // A part the option's picture lacks, or an option the item lacks, reads as nothing.
        assert_eq!(pill(&options, &Taken { option: 1, part: "header".into() }), None);
        assert_eq!(pill(&options, &Taken { option: 7, part: "header".into() }), None);
    }

    #[test]
    fn send_mix_posts_a_body_that_reads_back_as_the_ipc_type() {
        let taken = vec![Taken { option: 0, part: "header".into() }, Taken { option: 1, part: "sidebar".into() }];
        let posted = ipc::encode(&body("mock-01", "ask-layout", &taken)).expect("encodes");
        let read: AnswerWaitingParts = ipc::decode("AnswerWaitingParts", posted.as_bytes()).expect("reads back");
        assert_eq!(read.session_id.as_str(), "mock-01");
        assert_eq!(read.item_id, "ask-layout");
        assert_eq!(read.parts_picked.iter().map(|one| (one.option, one.part.as_str())).collect::<Vec<_>>(), [(0, "header"), (1, "sidebar")]);
        let json: serde_json::Value = serde_json::from_str(&posted).expect("is JSON");
        assert_eq!(json["parts_picked"][1], serde_json::json!({"option": 1, "part": "sidebar"}));
    }

    #[test]
    fn also_offers_only_the_forms_the_job_produced() {
        assert_eq!(also(Produced::default()), []);
        assert_eq!(also(Produced { summary: true, ..Produced::default() }), [Form::Listen]);
        assert_eq!(also(Produced { page: true, picture: true, summary: true }), [Form::Page, Form::Picture, Form::Listen]);
        assert_eq!(also(Produced { page: false, picture: true, summary: false }), [Form::Picture]);
    }
}
