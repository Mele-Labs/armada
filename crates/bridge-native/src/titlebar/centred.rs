//! The notch stands on the window's middle whatever the picker reads, and never over the start or the end.

use std::sync::Arc;

use gpui::{div, prelude::*, px, size, Bounds, Context, Entity, Pixels, TestAppContext, TestDispatcher, Window};

use super::{scenes, TitleBar};

struct Holder(Entity<TitleBar>);

impl Render for Holder {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().flex_col().child(self.0.clone())
    }
}

/// Where the bar draws its start, its notch and its end in a window `width` wide, the project named `name`.
fn drawn(width: f32, name: &str) -> [Bounds<Pixels>; 4] {
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, Arc::new(gpui_apple::AppleTextSystem::new()));
    cx.update(crate::init_app);
    let mut face = scenes::rest();
    face.projects[0].name = name.to_string().into();
    cx.update(|cx| cx.set_global(scenes::Opening(face)));
    let (holder, cx) = cx.add_window_view(|window, cx| Holder(cx.new(|cx| TitleBar::new(window, cx))));
    cx.simulate_resize(size(px(width), px(240.)));
    for _ in 0..2 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    cx.update(|_, cx| {
        let at = &holder.read(cx).0.read(cx).at;
        [at.start.get(), at.centre.get(), at.end.get(), at.row.get()].map(|one| one.expect("drawn"))
    })
}

const SHORT: &str = "Armada";
const LONG: &str = "Storefront checkout and pricing";

#[test]
fn the_notch_stands_on_the_windows_middle_whatever_the_picker_reads() {
    for width in [1440., 1800., 2560.] {
        for name in [SHORT, LONG] {
            let [start, centre, end, row] = drawn(width, name);
            let off = (centre.center().x - px(width / 2.)).abs();
            assert!(off <= px(1.), "at {width} with \"{name}\" the notch's middle is {off:?} from the window's: {start:?} {centre:?} {end:?} {row:?}");
        }
    }
}

#[test]
fn short_of_room_the_start_yields_and_nothing_is_drawn_under_the_notch() {
    for width in [1000., 1200.] {
        for name in [SHORT, LONG] {
            let [start, centre, end, _] = drawn(width, name);
            assert!(start.right() <= centre.left(), "at {width} with \"{name}\" the start runs under the notch: {start:?} {centre:?}");
            assert!(centre.right() <= end.left(), "at {width} with \"{name}\" the end runs under the notch: {centre:?} {end:?}");
            assert!(end.right() <= px(width), "at {width} with \"{name}\" the end runs off the window: {end:?}");
        }
    }
}
