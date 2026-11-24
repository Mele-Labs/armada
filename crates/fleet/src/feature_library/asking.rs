//! Asking the librarian: his words from the Ask box, one model call, and at most one act, which Undo takes back.
//! `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! **Only on his press, and spent from a night of its own** (`features.librarianAskShare`), counted in the store so a
//! restart does not reset it. What Undo needs is held here in memory for the last asks of each project, and a restart
//! forgets it: the act stays as it was done.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AskLibrarian, BoardLayout, Instant, LibrarianAct, LibrarianAnswered, LibrarianAskUndone, RankWithLibrarian, RankedIssues, RefileLibrarianAsk, SaveProjectBoard, UndoLibrarianAsk};
use store::{feature_link_kind, feature_state, KeptFeatureLink};

use super::asking_rank::{self, Ranking};
use super::asking_reply::{self, Named, Said, Screen};
use super::reading::Loaded;
use crate::daemon::Fleet;

/// Blank words and no files.
const BLANK: &str = "fleet.librarian_ask_blank";
/// The night's asks are spent.
const SPENT: &str = "fleet.librarian_asks_spent";
/// An ask Fleet does not hold, or one with nothing to undo or move.
const ASK_UNKNOWN: &str = "fleet.librarian_ask_unknown";
/// How many asks of a project Undo reaches back to.
const MOST_HELD: usize = 20;
/// How a link the librarian filed was found, as `feature_links.via` holds it.
const FILED_VIA: &str = "librarian";

/// What taking an ask back puts back.
#[derive(Clone, Debug)]
enum Undo {
    /// The layout kept before, or `None` where the preset was drawn, so the kept one is forgotten.
    Board { before: Option<BoardLayout> },
    /// The read it keeps, under the feature it is under now.
    Filed { feature_id: String, feature_title: String, target: String, title: String },
    /// The ranking it made, by when.
    Ranked { at: Instant },
}

#[derive(Default)]
struct OfProject {
    asks: VecDeque<(String, Undo)>,
    ranking: Option<Ranking>,
}

/// Each project's last asks, for Undo and Move, and the librarian's ranking of its might-lists. In memory.
#[derive(Default)]
pub(crate) struct Asks(Mutex<BTreeMap<String, OfProject>>);

impl Asks {
    fn with<T>(&self, project: &str, act: impl FnOnce(&mut OfProject) -> T) -> T {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        act(held.entry(project.to_string()).or_default())
    }

    fn keep(&self, project: &str, at: &str, undo: Undo) {
        self.with(project, |held| {
            held.asks.retain(|(one, _)| one != at);
            held.asks.push_back((at.to_string(), undo));
            while held.asks.len() > MOST_HELD {
                held.asks.pop_front();
            }
        });
    }

    fn held(&self, project: &str, at: &str) -> Option<Undo> {
        self.with(project, |held| held.asks.iter().find(|(one, _)| one == at).map(|(_, undo)| undo.clone()))
    }

    fn let_go(&self, project: &str, at: &str) {
        self.with(project, |held| held.asks.retain(|(one, _)| one != at));
    }

    fn replace(&self, project: &str, at: &str, undo: Undo) {
        self.with(project, |held| {
            if let Some((_, kept)) = held.asks.iter_mut().find(|(one, _)| one == at) {
                *kept = undo;
            }
        });
    }

    /// The ranking held for `project`, where one is.
    pub(crate) fn ranking(&self, project: &str) -> Option<Ranking> {
        self.with(project, |held| held.ranking.clone())
    }

    fn rank(&self, project: &str, ranking: Ranking) {
        self.with(project, |held| held.ranking = Some(ranking));
    }

    /// Let go of `project`'s ranking, or only the one made `at` where it is given.
    fn forget(&self, project: &str, at: Option<&Instant>) {
        self.with(project, |held| {
            if at.is_none_or(|at| held.ranking.as_ref().is_some_and(|ranking| ranking.at == *at)) {
                held.ranking = None;
            }
        });
    }
}

/// The reply where the model did not answer.
const NO_ANSWER: &str = "The librarian did not answer.";

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
    /// `project` trimmed, or the refusal for one Fleet does not hold.
    fn asked_project(&self, project: &str) -> Result<String, Refusal> {
        let project = project.trim();
        if self.seed_roots(project).is_none() {
            return Err(self.hosted_refusal(super::seeding::PROJECT, &format!("no project is named `{project}`")));
        }
        Ok(project.to_string())
    }

    /// The calls `project`'s asks may still make on `night`, refused where none is left.
    async fn ask_calls_left(&self, project: &str, night: &str, limit: u32) -> Result<u32, Refusal> {
        let spent = self.store().lock().await.librarian_ask_calls(project, night).map_err(|why| self.ledger_fault(why))?;
        if spent >= limit {
            let message = format!("Asks of the librarian are spent for tonight: {limit} model calls. They reset at local midnight.");
            return Err(Refusal::IllegalMove(ipc::WireError::raised(SPENT, message, self.run_id())));
        }
        Ok(limit - spent)
    }

    async fn ask_call_spent(&self, project: &str, night: &str) -> Result<u32, Refusal> {
        self.store().lock().await.spend_librarian_ask_call(project, night).map_err(|why| self.ledger_fault(why))
    }

    /// `ask_librarian`.
    pub(super) async fn librarian_asked(&self, caller: Option<api::Caller>, ask: AskLibrarian) -> Result<LibrarianAnswered, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&ask.project)?;
        let files: Vec<String> = ask.files.iter().map(|one| one.trim().to_string()).filter(|one| !one.is_empty()).collect();
        if ask.words.trim().is_empty() && files.is_empty() {
            return Err(self.hosted_refusal(BLANK, "an ask of the librarian needs words or a file"));
        }
        let screen = Screen::of(&ask.context);
        let limit = self.librarian_set().ask_share;
        let night = self.librarian_night().await;
        self.ask_calls_left(&project, &night, limit).await?;
        let loaded = Loaded::read(&*self.store().lock().await, &project).map_err(|why| self.ledger_fault(why))?;
        let features: Vec<Named> = loaded
            .map
            .features()
            .iter()
            .filter(|feature| feature.state != feature_state::PENDING)
            .map(|feature| Named {
                slug: feature.slug.clone(),
                title: feature.title.clone(),
                area: loaded.areas.iter().find(|area| area.slug == feature.area).map_or_else(|| feature.area.clone(), |area| area.title.clone()),
            })
            .collect();
        let board = ask.board.as_ref().filter(|_| screen == Screen::Dashboard);
        let asked_at = Instant::carried(self.now().as_str());
        let question = asking_reply::question(&screen, &ask.words, &files, &features, board);
        self.ask_call_spent(&project, &night).await?;
        let reading = match self.story_asked(&question).await {
            Ok(reply) => asking_reply::read(&reply, &screen),
            Err(why) => {
                eprintln!("fleet: an ask of the librarian for project {project} was not answered: {why}");
                return self.librarian_answered(&project, asked_at, NO_ANSWER.to_string(), None, &night, limit).await;
            }
        };
        let said_or = |fallback: &str| if reading.reply.is_empty() { fallback.to_string() } else { reading.reply.clone() };
        let (reply, act, undo) = match reading.said.clone() {
            Said::Answer => (said_or(NO_ANSWER), None, None),
            Said::Board(blocks) => match (board, blocks) {
                (Some(board), Some(blocks)) if asking_reply::fits(&blocks, board) => {
                    let layout = BoardLayout { blocks };
                    let before = (!board.preset).then(|| board.layout.clone());
                    self.project_board_saved(project.clone(), SaveProjectBoard { layout: Some(layout.clone()) }).await?;
                    (said_or("The board was changed."), Some(LibrarianAct::Board { layout }), Some(Undo::Board { before }))
                }
                _ => ("The board was not changed. The librarian's layout did not fit the board.".to_string(), None, None),
            },
            Said::File { target, title, feature } => {
                let given = files.iter().any(|one| *one == target) || ask.words.contains(target.as_str());
                let default = match &screen {
                    Screen::Feature(slug) => Some(slug.as_str()),
                    _ => None,
                };
                let under = loaded.feature_named(feature.trim()).or_else(|| default.and_then(|slug| loaded.feature_named(slug)));
                match (given, under) {
                    (true, Some(under)) => {
                        let title = Some(asking_reply::one_line(&title, 120)).filter(|title| !title.is_empty()).unwrap_or_else(|| target.clone());
                        let filed = Undo::Filed { feature_id: under.id.clone(), feature_title: under.title.clone(), target: target.clone(), title: title.clone() };
                        self.filed_kept(&filed, asked_at.as_str()).await?;
                        let act = LibrarianAct::FiledDoc { target, title: title.clone(), feature: under.slug.clone(), feature_title: under.title.clone() };
                        (said_or(&format!("Filed {title} under {}.", under.title)), Some(act), Some(filed))
                    }
                    _ => ("Nothing was filed. The librarian named a file you did not give, or no feature of this project.".to_string(), None, None),
                }
            }
            Said::Rank => match self.librarian_rank_called(&project, &night, limit).await {
                Ok(Some(at)) => (said_or("The lists were ranked."), Some(LibrarianAct::Ranked), Some(Undo::Ranked { at })),
                Ok(None) => ("The lists were not ranked. The librarian gave no order.".to_string(), None, None),
                Err(refusal) if refusal.error().code == SPENT => ("The lists were not ranked. Tonight's asks are spent.".to_string(), None, None),
                Err(_) => ("The lists were not ranked. They could not be read.".to_string(), None, None),
            },
        };
        if let Some(undo) = undo {
            self.librarian_asks().keep(&project, asked_at.as_str(), undo);
        }
        self.librarian_answered(&project, asked_at, reply, act, &night, limit).await
    }

    async fn librarian_answered(&self, project: &str, asked_at: Instant, reply: String, act: Option<LibrarianAct>, night: &str, limit: u32) -> Result<LibrarianAnswered, Refusal> {
        let calls = self.store().lock().await.librarian_ask_calls(project, night).map_err(|why| self.ledger_fault(why))?;
        Ok(LibrarianAnswered { project: project.to_string(), asked_at, reply, act, calls, limit })
    }

    /// The read a filed ask keeps, as a doc of its feature.
    async fn filed_kept(&self, filed: &Undo, at: &str) -> Result<(), Refusal> {
        let Undo::Filed { feature_id, target, title, .. } = filed else { return Ok(()) };
        let link = KeptFeatureLink {
            feature_id: feature_id.clone(),
            kind: feature_link_kind::DOC.to_string(),
            target: target.clone(),
            job: String::new(),
            title: title.clone(),
            state: String::new(),
            via: FILED_VIA.to_string(),
            at: at.to_string(),
            gist: String::new(),
            gist_by: String::new(),
        };
        self.store().lock().await.put_feature_link(&link).map(drop).map_err(|why| self.ledger_fault(why))
    }

    async fn filed_dropped(&self, feature_id: &str, target: &str) -> Result<(), Refusal> {
        self.store().lock().await.drop_feature_link(feature_id, feature_link_kind::DOC, target, "").map(drop).map_err(|why| self.ledger_fault(why))
    }

    fn ask_unknown(&self, project: &str, at: &Instant) -> Refusal {
        Refusal::NotFound(ipc::WireError::raised(ASK_UNKNOWN, format!("no ask of project `{project}` at {} to undo", at.as_str()), self.run_id()))
    }

    /// `undo_librarian_ask`. **An ask is undone once**: Fleet lets go of it after.
    pub(super) async fn librarian_ask_undone(&self, caller: Option<api::Caller>, undo: UndoLibrarianAsk) -> Result<LibrarianAskUndone, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&undo.project)?;
        let Some(held) = self.librarian_asks().held(&project, undo.asked_at.as_str()) else {
            return Err(self.ask_unknown(&project, &undo.asked_at));
        };
        let reply = match &held {
            Undo::Board { before } => {
                self.project_board_saved(project.clone(), SaveProjectBoard { layout: before.clone() }).await?;
                match before {
                    None => "The board is the preset again.".to_string(),
                    Some(_) => "The board is back as it was.".to_string(),
                }
            }
            Undo::Filed { feature_id, target, title, feature_title, .. } => {
                self.filed_dropped(feature_id, target).await?;
                format!("{title} is no longer filed under {feature_title}.")
            }
            Undo::Ranked { at } => {
                self.librarian_asks().forget(&project, Some(at));
                "The lists are back in feature order.".to_string()
            }
        };
        self.librarian_asks().let_go(&project, undo.asked_at.as_str());
        Ok(LibrarianAskUndone { project, asked_at: undo.asked_at, reply })
    }

    /// `refile_librarian_ask`. **No model call.**
    pub(super) async fn librarian_ask_refiled(&self, caller: Option<api::Caller>, refile: RefileLibrarianAsk) -> Result<LibrarianAnswered, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&refile.project)?;
        let Some(Undo::Filed { feature_id, target, title, .. }) = self.librarian_asks().held(&project, refile.asked_at.as_str()) else {
            return Err(self.ask_unknown(&project, &refile.asked_at));
        };
        let loaded = Loaded::read(&*self.store().lock().await, &project).map_err(|why| self.ledger_fault(why))?;
        let slug = refile.feature.trim();
        let under = loaded.feature_named(slug).ok_or_else(|| self.feature_unknown(slug))?;
        let moved = Undo::Filed { feature_id: under.id.clone(), feature_title: under.title.clone(), target: target.clone(), title: title.clone() };
        if feature_id != under.id {
            self.filed_dropped(&feature_id, &target).await?;
            self.filed_kept(&moved, self.now().as_str()).await?;
            self.librarian_asks().replace(&project, refile.asked_at.as_str(), moved);
        }
        let act = LibrarianAct::FiledDoc { target: target.clone(), title: title.clone(), feature: under.slug.clone(), feature_title: under.title.clone() };
        let reply = format!("Filed {title} under {}.", under.title);
        let limit = self.librarian_set().ask_share;
        let night = self.librarian_night().await;
        self.librarian_answered(&project, refile.asked_at, reply, Some(act), &night, limit).await
    }

    /// `rank_with_librarian`.
    pub(super) async fn librarian_rank_pressed(&self, caller: Option<api::Caller>, rank: RankWithLibrarian) -> Result<RankedIssues, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&rank.project)?;
        let limit = self.librarian_set().ask_share;
        let night = self.librarian_night().await;
        self.librarian_rank_called(&project, &night, limit).await?;
        self.ranked_issues_read(project).await
    }

    /// `forget_librarian_ranking`.
    pub(super) async fn librarian_ranking_forgotten(&self, caller: Option<api::Caller>, forget: RankWithLibrarian) -> Result<RankedIssues, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&forget.project)?;
        self.librarian_asks().forget(&project, None);
        self.ranked_issues_read(project).await
    }

    /// One rank call for `project`, spent from the night's asks: when the ranking it now holds was made, or `None` where
    /// the lists are empty, or the model failed or named no row. **Empty lists ask nothing and spend nothing.**
    async fn librarian_rank_called(&self, project: &str, night: &str, limit: u32) -> Result<Option<Instant>, Refusal> {
        self.ask_calls_left(project, night, limit).await?;
        let lists = self.ranked_by_features(project.to_string()).await?;
        if lists.might_pick_up.is_empty() && lists.might_be_interested.is_empty() {
            return Ok(None);
        }
        let worked = {
            let now = self.now().epoch_millis().unwrap_or(0);
            let store = self.store().lock().await;
            let loaded = Loaded::read(&store, project).map_err(|why| self.ledger_fault(why))?;
            let times = store.feature_job_times(project).map_err(|why| self.ledger_fault(why))?;
            let recent = super::ranking::recent_jobs(&times, now);
            let mut worked: Vec<(String, u32)> =
                loaded.map.features().iter().filter_map(|feature| recent.get(&feature.id).map(|jobs| (feature.title.clone(), u32::try_from(jobs.len()).unwrap_or(u32::MAX)))).collect();
            worked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            worked
        };
        self.ask_call_spent(project, night).await?;
        let at = Instant::carried(self.now().as_str());
        match self.story_asked(&asking_rank::question(&lists, &worked)).await {
            Ok(reply) => Ok(asking_rank::read(&reply, &lists, at.clone()).map(|ranking| {
                self.librarian_asks().rank(project, ranking);
                at
            })),
            Err(why) => {
                eprintln!("fleet: the librarian's ranking for project {project} was not made: {why}");
                Ok(None)
            }
        }
    }
}
