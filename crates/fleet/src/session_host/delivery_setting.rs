//! The delivery question answered by a setting, and a setting learned from an answer.
//! `docs/concepts/session/the-delivery-question.md`.
//!
//! **Read live, from the Job's project over Kit**: a setting moved since the Job started is the one that answers.
//! No layer setting it is the question, and the first answer in a project then becomes its value.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Projects as _;
use config::ResolvedValue;
use core_model::{DeliverySetting, Layer, ScopeId};
use ipc::{DeliveryChoice, EditProject, ManifestId};

use crate::daemon::Fleet;

/// What a Job's delivery came to once its first message was read.
pub(crate) enum DeliveryKept {
    /// The question stands.
    Asked,
    /// The first message named it.
    Named,
    /// A setting answered it. The words are what the agent is told.
    BySetting(String),
}

/// The Job's delivery setting, and the project it was read for.
struct SettingRead {
    project: Option<String>,
    setting: ResolvedValue<DeliverySetting>,
}

const WHERE_CHANGED: &str = "Change it in Settings, in the project's settings.";

/// What the agent is told of a landing a setting chose.
fn setting_words(choice: DeliveryChoice, project: bool) -> String {
    let whose = if project { "this project's setting" } else { "the delivery setting" };
    format!("Fleet: {whose} is {}. {}", choice.label(), choice.detail())
}

/// The choice a setting makes. `Ask` makes none.
fn choice_of(setting: DeliverySetting) -> Option<DeliveryChoice> {
    match setting {
        DeliverySetting::AutoLand => Some(DeliveryChoice::AutoLand),
        DeliverySetting::OpenPr => Some(DeliveryChoice::OpenPr),
        DeliverySetting::KeepLocal => Some(DeliveryChoice::KeepLocal),
        DeliverySetting::Ask => None,
    }
}

fn setting_of(choice: DeliveryChoice) -> DeliverySetting {
    match choice {
        DeliveryChoice::AutoLand => DeliverySetting::AutoLand,
        DeliveryChoice::OpenPr => DeliverySetting::OpenPr,
        DeliveryChoice::KeepLocal => DeliverySetting::KeepLocal,
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The delivery setting for Job `id`, `None` where it cannot be read: **a fault is the question**, and learns nothing.
    /// A project that does not read falls back to no project, as an agent start does.
    async fn delivery_setting_read(&self, id: &str) -> Option<SettingRead> {
        let hosting = self.store().lock().await.hosting(id).ok().flatten()?;
        let repository = ManifestId::carried(&hosting.manifest_id);
        let project = self.project_of_job(id).await;
        if let Ok(resolved) = self.values_resolved(project.as_deref(), Some(&repository)).await {
            return Some(SettingRead { project, setting: resolved.delivery });
        }
        if project.is_none() {
            return None;
        }
        let resolved = self.values_resolved(None, Some(&repository)).await.ok()?;
        Some(SettingRead { project: None, setting: resolved.delivery })
    }

    fn project_title(&self, id: &str) -> Option<String> {
        let held = self.projects_held();
        ScopeId::new(id).ok().and_then(|scope| held.get(&scope).map(|project| project.name().as_str().to_string()))
    }

    /// The choice a setting makes for Job `id`, with what the person is told of it and the agent's words.
    /// `None` where no layer sets one, or it is `ask`.
    pub(crate) async fn delivery_by_setting(&self, id: &str) -> Option<(DeliveryChoice, String, String)> {
        let read = self.delivery_setting_read(id).await?;
        let choice = read.setting.value.and_then(choice_of)?;
        let words = setting_words(choice, read.setting.from == Layer::Project);
        let said = match (read.setting.from, read.project.as_deref().and_then(|id| self.project_title(id))) {
            (Layer::Project, Some(name)) => format!("{}, as {name} is set. {WHERE_CHANGED}", choice.label()),
            _ => format!("{}, as set in Settings. {WHERE_CHANGED}", choice.label()),
        };
        Some((choice, said, words))
    }

    /// Keep `choice` as the value of the Job's project, where it is in one and no layer sets the key. **`ask` anywhere
    /// is the person turning this off**, so it writes nothing. Returns what the person is told of it. A failed write is
    /// said on the log and never undoes the answer.
    pub(crate) async fn delivery_learned(self: &Arc<Self>, id: &str, choice: DeliveryChoice) -> Option<String> {
        let read = self.delivery_setting_read(id).await?;
        let project = read.project?;
        if read.setting.value.is_some() {
            return None;
        }
        let held = self.project_read(&project).ok()?;
        let name = held.name().as_str().to_string();
        let mut set = crate::values::values_set(held.values()).unwrap_or_default();
        set.delivery = Some(setting_of(choice).as_wire().to_string());
        let edit = EditProject { values: Some(set), ..EditProject::default() };
        if let Err(refusal) = self.edit_project(project, edit).await {
            self.values_warned("how a project's work lands was not kept", Some(id), &refusal.error().message);
            return None;
        }
        Some(format!("Kept for {name}: its Jobs land this way from now on. {WHERE_CHANGED}"))
    }
}
