//! The floor plan, as a block of the Cockpit's home: the plan alone, as many columns as its own width holds, with no header and no
//! Switch on (the home's strip holds that). The feature map where the project's catalog holds features, else the code map of its
//! modules. It reads the library the Features screen already holds (`FeaturesScreen::watch`) and one overview read for the code
//! map, and draws; the chip on a feature joins its Job to the Jobs list's record, so its hue is the one that Job's group wears
//! there, never a field of the wire. The code map has no doors: Fleet has no project-wide floor plan to draw them from.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Context, Entity, FontWeight, Render, Role, SharedString, Subscription, Task, Window};
use ipc::{FeatureLibrary, FloorPlan, FloorRoom, MapRoom, ProjectOverview, SessionRecord};

use super::dossier::{dot, hue_among, hue_square, tint};
use super::floor_map::Laid;
use super::{curate, mock, wire, FeaturesScreen, Load, Source};
use crate::jobs::JobsList;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::operator::view::chip;
use crate::settings::wire::Refused;
use crate::titlebar::parts::{sans, tip};

/// A Job on a feature: its title, where its state stands and the hue that state wears in the Jobs list.
#[derive(Clone, Debug, PartialEq)]
pub struct JobChip {
    /// The Job's id where the Jobs list holds it, which is what a press opens.
    pub open: Option<String>,
    pub title: String,
    pub said: &'static str,
    pub hue: Token,
}

/// A feature of the catalog as the map draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub slug: String,
    pub title: String,
    pub stale: bool,
    pub jobs: Vec<JobChip>,
}

/// An area as the map draws it: a card with its hue and its features.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub slug: String,
    pub title: String,
    pub hue: Token,
    pub places: Vec<Place>,
}

/// The areas of the catalog that hold a feature, in Fleet's order, each feature with the Jobs on it joined to `jobs`. A feature
/// still pending is no part of the catalog yet.
pub fn cards(library: &FeatureLibrary, jobs: &[SessionRecord]) -> Vec<Card> {
    let slugs: Vec<&str> = library.areas.iter().map(|area| area.slug.as_str()).collect();
    library
        .areas
        .iter()
        .filter(|area| area.slug != curate::PENDING_AREA)
        .filter_map(|area| {
            let places: Vec<Place> = area
                .features
                .iter()
                .filter(|row| !curate::is_pending(row))
                .map(|row| Place {
                    slug: row.slug.clone(),
                    title: row.title.clone(),
                    stale: row.stale_since.is_some(),
                    jobs: row
                        .jobs_now
                        .iter()
                        .map(|on| match jobs.iter().find(|job| job.id.as_str() == on.session_id.as_str()) {
                            Some(job) => {
                                let (said, hue) = crate::jobs::stand_of(job);
                                JobChip { open: Some(job.id.as_str().to_string()), title: crate::jobs::titled(job.title.as_deref()), said, hue }
                            }
                            None => JobChip { open: None, title: on.title.clone(), said: "On it now", hue: token::FG_MUTED },
                        })
                        .collect(),
                })
                .collect();
            (!places.is_empty()).then(|| Card { slug: area.slug.clone(), title: area.title.clone(), hue: hue_among(slugs.iter().copied(), &area.slug), places })
        })
        .collect()
}

/// The project's rooms as a floor plan to lay out: no files, so each plot is the same size, and no doors.
pub fn code_map(overview: &ProjectOverview) -> FloorPlan {
    let room = |room: &MapRoom| FloorRoom { module: room.module.clone(), path: room.path.clone(), kind: room.kind.clone(), lit: room.area.is_some(), manifest_id: room.manifest_id.clone(), files: Vec::new() };
    FloorPlan { rooms: overview.rooms.iter().map(room).collect(), doors: Vec::new() }
}

/// What the code map's read stands at.
enum Overview {
    Idle,
    Reading,
    Read(Rc<Laid>),
    Failed(String),
}

pub struct Floor {
    features: Entity<FeaturesScreen>,
    list: Entity<JobsList>,
    overview: Overview,
    /// The project the overview was asked for, so one read is made for it and no more.
    asked: Option<String>,
    work: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl Floor {
    /// The plan alone, for a block of the Cockpit's home: it reads the library as soon as it is made and follows it from then on.
    pub fn plan(features: Entity<FeaturesScreen>, list: Entity<JobsList>, cx: &mut Context<Self>) -> Self {
        let mut this = Self::waiting(features, list, cx);
        this.follow(cx);
        this
    }

    /// The plan alone, asking nothing of Fleet until `follow`: for a home that is not drawn yet.
    pub fn waiting(features: Entity<FeaturesScreen>, list: Entity<JobsList>, cx: &mut Context<Self>) -> Self {
        let subs = vec![cx.observe(&features, |this, _, cx| this.sync(cx)), cx.observe(&list, |_, _, cx| cx.notify())];
        Floor { features, list, overview: Overview::Idle, asked: None, work: None, _subs: subs }
    }

    /// The floor reads the library without being in front. Called again where another place was in front and left.
    pub fn follow(&mut self, cx: &mut Context<Self>) {
        self.features.update(cx, |features, cx| features.watch(cx));
        self.sync(cx);
    }

    /// The code map is read for the project whose catalog holds no feature, once for each.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let features = self.features.read(cx);
        let wants = match &features.library {
            Load::Read(library) if cards(library, &[]).is_empty() => features.floor_project(cx),
            _ => None,
        };
        if wants != self.asked {
            self.asked = wants.clone();
            self.work = None;
            self.overview = Overview::Idle;
            if let Some(project) = wants {
                self.read(project, cx);
            }
        }
        cx.notify();
    }

    fn read(&mut self, project: String, cx: &mut Context<Self>) {
        let ask = match &self.features.read(cx).source {
            Source::Mock => {
                self.overview = Overview::Read(Rc::new(Laid::new(code_map(&mock::overview()))));
                return;
            }
            Source::Live(fleet) => fleet.read(cx).held().port().map(|port| wire::overview(port, &project, cx)),
        };
        let Some(ask) = ask else {
            self.overview = Overview::Failed("Fleet is not connected".into());
            return;
        };
        self.overview = Overview::Reading;
        self.work = Some(cx.spawn(async move |this, cx| {
            let answer: Result<ProjectOverview, Refused> = ask.await;
            let _ = this.update(cx, |this, cx| {
                this.overview = match answer {
                    Ok(overview) => Overview::Read(Rc::new(Laid::new(code_map(&overview)))),
                    Err(why) => Overview::Failed(why.said),
                };
                cx.notify();
            });
        }));
    }

    // -- drawn ---------------------------------------------------------------------------------

    /// The key to the chips' colours: each state a Job on a feature can stand in, in its hue.
    fn legend(&self, look: &Look) -> gpui::Div {
        div().flex_1().min_w_0().flex().flex_wrap().items_center().justify_end().gap_x(look.length(token::SPACE_3)).children(crate::jobs::legend().into_iter().map(|(said, hue)| {
            div().flex_none().flex().items_center().gap(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(dot(look, hue, false)).child(said)
        }))
    }

    /// One chip for each project Fleet names, the one on the floor ringed: the Features screen's own pick, kept the same way.
    fn switcher(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let features = self.features.read(cx);
        let chips = features.chips(cx);
        if chips.is_empty() {
            return None;
        }
        let shown = features.shown_project(cx);
        let row: Vec<AnyElement> = chips
            .into_iter()
            .map(|(id, name)| {
                let on = shown.as_deref() == Some(id.as_str());
                let press = id.clone();
                let pill = super::projects::project_pill(look, format!("floor-project-{id}"), &name, on).on_click(cx.listener(move |this, _, _, cx| {
                    let project = press.clone();
                    this.features.update(cx, |features, cx| features.choose(project, cx));
                }));
                labelled(pill, Role::Button, name).into_any_element()
            })
            .collect();
        Some(div().id("floor-projects").flex_none().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).children(row).into_any_element())
    }

    /// A grey shape where an area will be, while the library is read.
    fn bones(&self, look: &Look) -> AnyElement {
        let bone = |wide: f32| div().h(look.length(token::LEADING_XS)).w(px(wide)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER));
        let card = |n: usize| {
            div()
                .id(SharedString::from(format!("floor-bone-{n}")))
                .flex_1()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_SUNKEN))
                .child(bone(96. + 12. * n as f32))
                .child(bone(140.))
                .child(bone(112.))
        };
        labelled(div().id("floor-loading").flex().gap(look.length(token::GAP_PANEL)).child(card(0)).child(card(1)).child(card(2)), Role::Image, "Loading the floor").into_any_element()
    }

    fn not_read(&self, why: &str, look: &Look) -> AnyElement {
        let said = format!("Not read: {why}");
        labelled(
            div().id("floor-not-read").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())),
            Role::Status,
            said,
        )
        .into_any_element()
    }

    /// The areas as cards, as many columns as the window has room for.
    fn feature_map(&self, cards: Vec<Card>, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let cards: Vec<AnyElement> = cards.into_iter().map(|card| self.area(card, look, cx)).collect();
        // A block is as wide as its box: the areas wrap to as many columns as that holds, never to the window's width.
        let least = look.length(token::SPACE_12) * 5.;
        let map = div().id("floor-feature-map").w_full().flex().flex_wrap().gap(look.length(token::GAP_PANEL)).children(cards.into_iter().map(|card| div().flex_grow(1.).flex_shrink(1.).flex_basis(least).max_w_full().min_w_0().child(card)));
        labelled(map, Role::Region, "Feature map").into_any_element()
    }

    /// An area: a header band in its hue over its features, each with the Jobs working on it.
    fn area(&self, card: Card, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let sunk = look.color(token::BG_SUNKEN);
        let band = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(tint(look, sunk, card.hue, 0.14))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(hue_square(look, card.hue, look.length(token::SPACE_2) + px(2.)))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("floor-area-{}", card.slug)))
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(card.hue))
                    .child(SharedString::from(card.title.clone())),
                Role::Heading,
                card.title.clone(),
            ));
        let rows: Vec<AnyElement> = card.places.into_iter().enumerate().map(|(at, place)| self.place(&card.slug, at, place, look, cx)).collect();
        labelled(
            div()
                .id(SharedString::from(format!("floor-card-{}", card.slug)))
                .flex()
                .flex_col()
                .min_w_0()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(sunk)
                .overflow_hidden()
                .child(band)
                .children(rows),
            Role::Group,
            card.title,
        )
        .into_any_element()
    }

    /// A feature's row: its title, a word where a section is stale, and a chip for each Job on it at the right.
    fn place(&self, area: &str, at: usize, place: Place, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let busy = !place.jobs.is_empty();
        let stale = place.stale.then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("floor-stale-{area}-{}", place.slug)))
                    .flex_none()
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                    .tooltip(tip("A written section is older than the last merge that changed its files"))
                    .child("stale"),
                Role::Image,
                "stale",
            )
        });
        let jobs: Vec<AnyElement> = place.jobs.into_iter().enumerate().map(|(n, job)| self.job_chip(area, &place.slug, n, job, look, cx)).collect();
        labelled(
            div()
                .id(SharedString::from(format!("floor-feature-{area}-{}", place.slug)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(if busy { token::FG_DEFAULT } else { token::FG_MUTED }))
                .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(place.title.clone())))
                .children(stale)
                .when(busy, |row| row.child(div().flex().flex_wrap().justify_end().gap(look.length(token::SPACE_1)).max_w(gpui::relative(0.66)).children(jobs))),
            Role::ListItem,
            place.title,
        )
        .into_any_element()
    }

    /// A Job on a feature: a pill filled in its state's hue, its title in it; a press opens the Job where the list holds it.
    fn job_chip(&self, area: &str, slug: &str, n: usize, job: JobChip, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let said = format!("{}, {}", job.title, job.said);
        let body = chip(job.title.clone(), look.color(job.hue), look.color(job.hue).opacity(0.16), look)
            .id(SharedString::from(format!("floor-job-{area}-{slug}-{n}")))
            .max_w_full()
            .min_w_0()
            .truncate()
            .tooltip(tip(said.clone()));
        match job.open {
            Some(id) => labelled(
                body.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                    this.list.update(cx, |list, cx| {
                        list.open_job(&id, cx);
                    });
                })),
                Role::Button,
                said,
            )
            .into_any_element(),
            None => labelled(body, Role::Group, said).into_any_element(),
        }
    }

    /// The project's rooms, as the plan of its floor: no doors.
    fn rooms(&self, look: &Look) -> AnyElement {
        let words = |said: &'static str| div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(said);
        let map = match &self.overview {
            Overview::Idle | Overview::Reading => self.bones(look),
            Overview::Failed(why) => self.not_read(why, look),
            Overview::Read(laid) if laid.plan.rooms.is_empty() => labelled(words("No modules read yet").id("floor-no-rooms"), Role::Status, "No modules read yet").into_any_element(),
            Overview::Read(laid) => super::page_view::floor_drawing(look, laid.clone(), true),
        };
        labelled(
            div().id("floor-code-map").w_full().flex().flex_col().gap(look.length(token::SPACE_3)).child(words("No features for this project yet. Its modules:")).child(map),
            Role::Region,
            "Code map",
        )
        .into_any_element()
    }
}

impl Render for Floor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let look = &look;
        let library = match &self.features.read(cx).library {
            Load::Reading => None,
            Load::Failed(why) => Some(Err(why.clone())),
            Load::Read(library) => Some(Ok(cards(library, self.list.read(cx).records()))),
        };
        let switcher = self.switcher(look, cx);
        let mapped = matches!(&library, Some(Ok(cards)) if !cards.is_empty());
        let body = match library {
            None => self.bones(look),
            Some(Err(why)) => self.not_read(&why, look),
            Some(Ok(cards)) if cards.is_empty() => self.rooms(look),
            Some(Ok(cards)) => self.feature_map(cards, look, cx),
        };
        // The project chips with the key to the chips' colours beside them, then the plan.
        let top = (switcher.is_some() || mapped).then(|| {
            div()
                .flex_none()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(look.length(token::SPACE_3))
                .gap_y(look.length(token::SPACE_2))
                .children(switcher)
                .children(mapped.then(|| self.legend(look).min_w(look.length(token::SPACE_12) * 3.)))
        });
        div()
            .id("home-floor-plan")
            .size_full()
            .flex()
            .flex_col()
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().id("floor-body").flex_1().min_h_0().overflow_y_scroll().p(look.length(token::SPACE_3)).flex().flex_col().gap(look.length(token::SPACE_3)).children(top).child(body))
            .into_any_element()
    }
}
