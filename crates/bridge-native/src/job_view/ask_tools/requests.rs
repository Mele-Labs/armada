//! API Client's own items in the tools list: → on its row opens them in the same list, New request first and then every saved
//! request, with a search over them that takes the keys. A pick is a chip of kind `request`.

use gpui::{div, prelude::*, svg, AnyElement, Role, SharedString};
use ipc::{Requests, TagKind};

use super::{API_CLIENT, NEW_REQUEST};
use crate::assets::icon_path;
use crate::job_view::offer::{Pick, Row};
use crate::labels::labelled;
use crate::look::{token, Look};

/// One saved request as the list shows it: where it is kept, its method, its name and its collection's.
#[derive(Clone, Debug, PartialEq)]
struct Saved {
    id: String,
    name: String,
    method: String,
    collection: String,
}

/// The saved requests of the Job's repository, which the search narrows.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Within {
    saved: Vec<Saved>,
}

impl Within {
    pub(crate) fn of(read: &Requests) -> Within {
        let saved = read
            .collections
            .iter()
            .flat_map(|placed| {
                let collection = &placed.item;
                collection.requests.iter().map(|request| Saved {
                    id: format!("{}/{}", collection.id, request.id),
                    name: request.name.clone(),
                    method: request.method.to_uppercase(),
                    collection: collection.name.clone(),
                })
            })
            .collect();
        Within { saved }
    }

    /// The method of the saved request a row picks; none for New request.
    pub(crate) fn method_of(&self, pick: &Pick) -> Option<&str> {
        let Pick::Tag { kind: TagKind::Request, id, .. } = pick else { return None };
        self.saved.iter().find(|one| one.id == *id).map(|one| one.method.as_str())
    }

    /// New request, then each saved request the search names by its name, method or collection, in the order Fleet lists them.
    /// New request is named by its own words too, so the search finds it.
    pub(crate) fn rows(&self, query: &str) -> Vec<Row> {
        let query = query.trim().to_lowercase();
        let named = |field: &str| query.is_empty() || field.to_lowercase().contains(&query);
        let new = named(NEW_REQUEST).then(|| Row {
            group: None,
            glyph: "plus",
            name: NEW_REQUEST.to_string(),
            says: String::new(),
            pick: Pick::Tag { kind: TagKind::Request, id: "new".to_string(), title: NEW_REQUEST.to_string(), url: None },
        });
        let saved = self.saved.iter().filter(|one| named(&one.name) || named(&one.method) || named(&one.collection)).map(|one| Row {
            group: None,
            glyph: "",
            name: one.name.clone(),
            says: one.collection.clone(),
            pick: Pick::Tag { kind: TagKind::Request, id: one.id.clone(), title: one.name.clone(), url: None },
        });
        new.into_iter().chain(saved).collect()
    }
}

/// The search over the items, which holds what was typed and takes the keys the box would.
pub(crate) fn search(look: &Look, query: &str) -> AnyElement {
    let edge = look.color(token::ACCENT);
    labelled(
        div()
            .id("offer-search")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .mx(look.length(token::SPACE_1))
            .mb(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_BASE))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .child(svg().path(icon_path("search")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(query.to_string()))
                    .child(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_3)).bg(edge)),
            ),
        Role::SearchInput,
        "Search API Client",
    )
    .into_any_element()
}

/// The column in front of a row's name: a saved request's method in the tool's hue, or New request's glyph.
pub(crate) fn lead(look: &Look, method: Option<&str>, glyph: &'static str) -> AnyElement {
    let hue = super::hue(API_CLIENT).map_or_else(|| look.color(token::FG_MUTED), |hue| look.color(hue));
    let column = div().flex_none().w(look.length(token::SPACE_12)).mr(look.length(token::SPACE_2)).flex().items_center();
    match method {
        Some(method) => column
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(hue)
            .when_some(look.font_stack(token::FONT_MONO).into_iter().next(), |column, mono| column.font_family(mono))
            .child(SharedString::from(method.to_string()))
            .into_any_element(),
        None => column.child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(hue)).into_any_element(),
    }
}
