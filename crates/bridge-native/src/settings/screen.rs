//! The app with Settings over it. It stands in front of the title bar's sink: Settings from the bar's menu and
//! ⌘, open it on the project of the Job on screen, or every project where none is, and the project switcher's
//! Settings opens it on that project's first page.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyView, App, Context, Entity, FocusHandle, Window};

use super::overlay::SettingsOverlay;
use super::wire::Wire;
use crate::fleet::FleetConnection;
use crate::titlebar::{self, Asked, AskedSink};

pub struct SettingsScreen {
    under: AnyView,
    pub overlay: Entity<SettingsOverlay>,
    /// Holds the keys anywhere in the app or in Settings, so a walk's keys go where a person's would.
    pub within: FocusHandle,
}

impl SettingsScreen {
    pub fn live(under: AnyView, fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new(under, Wire::Live(fleet.clone()), window, cx)
    }

    /// Over mock data, for a walk: the mock Fleet answers every read and save.
    pub fn mock(under: AnyView, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new(under, Wire::Mock(Box::new(super::mock::MockFleet::new())), window, cx)
    }

    /// For the walk of a refused save: the mock refuses this word as Fleet refuses one outside its set.
    pub fn refusing(&mut self, word: &str, cx: &mut Context<Self>) {
        self.overlay.update(cx, |overlay, _| {
            if let Wire::Mock(mock) = &mut overlay.wire {
                mock.refuses.push(word.to_string());
            }
        });
    }

    /// For the walk of choosing what to take from Armada: the copy of `feature` is what it began from, less what Armada changed.
    pub fn copied_from_base(&mut self, cx: &mut Context<Self>) {
        self.overlay.update(cx, |overlay, _| {
            if let Wire::Mock(mock) = &mut overlay.wire {
                mock.copied_from_base();
            }
        });
    }

    /// For the walk of a Mac Tailscale Funnel does not serve.
    pub fn without_funnel(&mut self, cx: &mut Context<Self>) {
        self.overlay.update(cx, |overlay, _| {
            if let Wire::Mock(mock) = &mut overlay.wire {
                mock.gateway = super::gateway::MockGateway::without_funnel();
            }
        });
    }

    fn new(under: AnyView, wire: Wire, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let overlay = cx.new(|cx| SettingsOverlay::new(wire, window, cx));
        let handle = window.window_handle();
        let me = overlay.downgrade();
        let told = me.clone();
        crate::job_view::on_job_shown(
            move |job, _, cx| {
                if let Some(me) = told.upgrade() {
                    me.update(cx, |this, _| this.job_shown = job);
                }
            },
            cx,
        );
        let behind = cx.try_global::<AskedSink>().cloned();
        cx.set_global(AskedSink(Rc::new(move |asked: &Asked, cx: &mut App| {
            let project = match asked {
                Asked::Settings => Some(None),
                Asked::RepositorySettings(_) => None,
                Asked::ProjectSettings(at) => Some(Some(*at)),
                _ => {
                    if let Some(behind) = &behind {
                        (behind.0)(asked, cx);
                    }
                    return;
                }
            };
            // The bar asks from inside its own update, so its projects are read once it is done.
            let me = me.clone();
            cx.defer(move |cx| {
                let Some(me) = me.upgrade() else { return };
                let scope = match project {
                    None => None,
                    // ⌘, and the bar's menu: the project the Job on screen was started for.
                    Some(None) => {
                        let this = me.read(cx);
                        this.job_shown.as_deref().and_then(|job| this.wire.project_of_job(job, cx))
                    }
                    Some(Some(at)) => match titlebar::live(cx).and_then(|bar| bar.read(cx).face.projects.get(at).map(|one| (one.id.to_string(), one.name.to_string()))) {
                        Some(project) => Some(project),
                        None => return,
                    },
                };
                let values = matches!(project, Some(Some(_)));
                let _ = handle.update(cx, move |_, window, cx| me.update(cx, |this, cx| this.show(scope, values, window, cx)));
            });
        })));
        SettingsScreen { under, overlay, within: cx.focus_handle() }
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().track_focus(&self.within).relative().size_full().child(self.under.clone()).child(div().absolute().top_0().left_0().child(self.overlay.clone()))
    }
}

impl Render for SettingsOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_overlay(window, cx)
    }
}
