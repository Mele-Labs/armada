//! The Job's Prototype tab: the scenes its agent saved, each drawn natively beside the thread.
//!
//! A prototype is an artifact holding a prototype part (`ipc::PartContent::Prototype`). Its tab
//! arrives the first time the part is seen and is read again each time Fleet lists the artifact
//! with a new revision, so the drawing follows each save. Its head names the scene, its states
//! and Pop out, which draws the same scene in a window of its own with the note capture over it.

use gpui::{div, prelude::*, AnyElement, Context, SharedString, Window};
use ipc::PartContent;

use super::row_tabs::{RowKind, RowTab};
use super::JobView;
use crate::dispatch::face::Offered;
use crate::keymap::Act;
use crate::prototype::{window, Home, PrototypeEvent, PrototypeView, Source};

/// The prototype part of an artifact, the first it holds.
fn part_of(artifact: &ipc::JobArtifact) -> Option<&ipc::PrototypePart> {
    artifact.parts.iter().find_map(|part| match &part.content {
        PartContent::Prototype(prototype) => Some(prototype),
        _ => None,
    })
}

impl JobView {
    /// What the Job's artifact `artifact` holds as a prototype: the tab's name and the scene to draw. An archived one has none.
    pub(super) fn prototype_source(&self, job: &str, artifact: &str) -> Option<(String, Source)> {
        let one = self.artifact_of(job, artifact).filter(|one| one.archived.is_none())?;
        let part = part_of(one)?;
        let title = crate::prototype::markup::parse(&part.markup).ok().map(|scene| scene.title).filter(|title| !title.is_empty()).unwrap_or_else(|| one.title.clone());
        Some((title, Source::from(part)))
    }

    /// The Job's prototypes, newest artifact first.
    fn prototypes_of(&self, job: &str) -> Vec<String> {
        self.artifact_rows(job).iter().filter(|one| one.archived.is_none() && part_of(one).is_some()).map(|one| one.id.clone()).collect()
    }

    /// Fleet listed the Job's artifacts: each scene is read again, the one first seen gets its tab, and a window popped out follows the save.
    pub(super) fn sync_prototypes(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        let mut arrived = false;
        for artifact in self.prototypes_of(job) {
            let Some((title, source)) = self.prototype_source(job, &artifact) else { continue };
            let held = self.row_tabs.get(job).and_then(|tabs| tabs.protos.get(&artifact)).map(|(view, _)| view.clone());
            match held {
                Some(view) => view.update(cx, |view, cx| view.set_source(source.clone(), cx)),
                None => {
                    let view = cx.new(|cx| PrototypeView::new(source.clone(), Home::Tab, cx));
                    let key = artifact.clone();
                    let popped = cx.subscribe(&view, move |this, _, event: &PrototypeEvent, cx| match event {
                        PrototypeEvent::PopOut => this.pop_out_prototype(&key, cx),
                    });
                    self.row_tabs.entry(job.clone()).or_default().protos.insert(artifact.clone(), (view, popped));
                }
            }
            if self.row_tabs.entry(job.clone()).or_default().arrive_prototype(&artifact, title) {
                arrived = true;
            }
            if let Some(tabs) = self.row_tabs.get_mut(job) {
                tabs.pops.retain(|(held, window)| *held != artifact || window.update(cx, |window, cx| window.set_source(source.clone(), cx)).is_ok());
            }
        }
        if arrived {
            self.sync_row_tabs_into_panes(job, cx);
        }
    }

    /// The Prototype tab from the + menu: the Job's newest scene, in front. A Job that has none opens nothing.
    pub(super) fn open_prototype_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(artifact) = self.prototypes_of(&job).into_iter().next() else { return };
        let Some((title, _)) = self.prototype_source(&job, &artifact) else { return };
        self.row_tabs.entry(job.clone()).or_default().focus_prototype(&artifact, title);
        self.row_tab_front(&job, RowKind::Prototype, &artifact, window, cx);
    }

    /// Whether the Job holds a scene the + menu can open.
    pub(super) fn has_prototype(&self, job: &str) -> bool {
        !self.prototypes_of(job).is_empty()
    }

    /// The scene in a window of its own, with the note capture over it. The notes go to this Job.
    pub(super) fn pop_out_prototype(&mut self, artifact: &str, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(source) = self.row_tabs.get(&job).and_then(|tabs| tabs.protos.get(artifact)).map(|(view, _)| view.read(cx).source().clone()) else { return };
        let title = self.record(&job, cx).and_then(|record| record.title.clone()).unwrap_or_default();
        let opened = window::open(source, Offered { id: job.clone(), title: title.into() }, self.fleet.clone(), cx);
        if let Some(opened) = opened {
            self.row_tabs.entry(job).or_default().pops.push((artifact.to_string(), opened));
        }
    }

    /// The tab's body: the scene under its head. The key for Pop out is answered here, wherever the focus is inside the tab.
    pub(super) fn prototype_body(&self, id: &SharedString, tab: &RowTab, cx: &Context<Self>) -> AnyElement {
        let Some(view) = self.row_tabs.get(id).and_then(|tabs| tabs.protos.get(&tab.key)).map(|(view, _)| view.clone()) else {
            return div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element();
        };
        let artifact = tab.key.clone();
        div()
            .id("row-tab-body")
            .key_context("prototype")
            .track_focus(&self.row_focus)
            .on_action(cx.listener(move |this, act: &Act, _, cx| {
                if act.is("prototype_pop_out") {
                    this.pop_out_prototype(&artifact, cx);
                } else {
                    cx.propagate();
                }
            }))
            .size_full()
            .child(view)
            .into_any_element()
    }
}
