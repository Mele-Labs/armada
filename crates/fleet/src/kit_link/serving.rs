//! Fleet's side of Kit over the Mac link: reads this Mac's pieces off the async workers, keeps what each linked Mac
//! offers and what the two last agreed, puts what crosses here, and tells Bridge and the linked Macs when it moved.

use std::path::Path;
use std::sync::{Arc, PoisonError};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{
    Event, HomeItem, HomeKind, HomeKindSwitch, HomeUnreadable, Instant, KitChanged, KitKeep, KitLink, KitLinkedMac, KitMark, KitOffer, KitSettled,
    MacNamed, OtherMacKit, SetKitLinkItem, SetKitLinkKind, SettleKitClash, WireError,
};
use sha2::{Digest, Sha256};

use super::{hash_of, item_of, judged, offer, plan, rows, Key, Mine, Other};
use crate::daemon::Fleet;
use crate::home::{kept_here, pieces, HomeFittings, Places, Repository, Syncs};

const KIT_NOT_KEPT: &str = "fleet.kit_link_not_kept";
/// A settle asked of a row that is not a clash with that Mac. A 409.
const NO_CLASH: &str = "fleet.kit_no_clash";

/// Whether a settle reached the Mac it was for.
pub(crate) enum SettleSent {
    /// It answered, taking it or refusing it: either way it is not sent again.
    Answered,
    Unreachable,
}

/// Everything a read needs before it leaves the async workers.
struct Context {
    fittings: Arc<HomeFittings>,
    servers: Vec<core_model::KitServer>,
    repositories: Vec<Repository>,
    syncs: Syncs,
    mods_dir: String,
}

impl Context {
    fn places(&self) -> Places<'_> {
        Places {
            mods: self.fittings.mods.as_ref(),
            kit: self.fittings.kit.as_ref(),
            harness: self.fittings.harness.as_ref(),
            servers: &self.servers,
            repositories: &self.repositories,
        }
    }

    /// Why `piece` stays here: a credential, or a mod `crate::mods` does not find valid.
    fn refused(&self, kind: HomeKind, piece: &pieces::Piece) -> Option<String> {
        if let Err(refused) = kept_here(piece) {
            return Some(refused.said());
        }
        if kind != HomeKind::Mods {
            return None;
        }
        let examined = crate::mods::examine(Path::new(&self.mods_dir), &piece.name);
        examined.problems.first().map(|problem| format!("The mod {} is not valid here: {problem}", piece.name))
    }
}

fn kept_item(key: &Key) -> store::KeptHomeItem {
    store::KeptHomeItem { kind: key.0.as_wire().to_string(), name: key.2.clone(), repository: key.1.clone() }
}

fn key_kept(kept: &store::KeptHomeItem) -> Option<Key> {
    Some((HomeKind::from_wire(&kept.kind)?, kept.repository.clone(), kept.name.clone()))
}

fn key_asked(item: &HomeItem) -> Key {
    (item.kind, item.repository.clone().filter(|_| item.kind == HomeKind::Memory), item.name.clone())
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
    fn kit_not_kept(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(KIT_NOT_KEPT, format!("What crosses to his other Macs could not be read or kept: {why}"), self.run_id()))
    }

    /// This Mac's name, as a clash names its side.
    fn kit_here_name(&self) -> String {
        crate::signing_in::this_machine(&self.machine_settings().now().get(keys::MACHINE_NAME))
    }

    /// What crosses on this Mac: every kind off until switched on.
    async fn kit_syncs(&self) -> Result<Syncs, Refusal> {
        let store = self.store().lock().await;
        let kinds = store.kit_link_kinds().map_err(|why| self.kit_not_kept(why))?;
        let items = store.kit_link_items().map_err(|why| self.kit_not_kept(why))?;
        Ok(Syncs {
            kinds: HomeKind::ALL.iter().copied().filter(|kind| kinds.get(kind.as_wire()).copied().unwrap_or(false)).collect(),
            items: items.iter().filter_map(key_kept).map(|key| item_of(&key)).collect(),
        })
    }

    /// `None` where this Fleet was started without Kit's folders: a test's. It still keeps and offers switches.
    async fn kit_context(&self, syncs: Syncs) -> Result<Option<Arc<Context>>, Refusal> {
        let Some(fittings) = self.home_held().fittings() else { return Ok(None) };
        // Not defaulted: a server read as missing here would read as only on the other Mac.
        let servers = self.store().lock().await.kit_servers().map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        let repositories = self
            .repositories()
            .every()
            .iter()
            .map(|one| Repository {
                name: Path::new(one.root()).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default(),
                root: one.root().to_string(),
            })
            .collect();
        Ok(Some(Arc::new(Context { fittings, servers, repositories, syncs, mods_dir: self.host().mods_dir.clone() })))
    }

    /// This Mac's pieces of what crosses, read again, with when each version was first read here.
    async fn kit_mine(&self, context: Option<&Arc<Context>>) -> Result<(Vec<Mine>, Vec<HomeUnreadable>), Refusal> {
        let Some(context) = context.cloned() else { return Ok((Vec::new(), Vec::new())) };
        let read = tokio::task::spawn_blocking(move || {
            let places = context.places();
            let (mut mine, mut unreadable) = (Vec::new(), Vec::new());
            for kind in context.syncs.looked_at() {
                let side = pieces::here(&places, kind);
                unreadable.extend(side.unreadable);
                for piece in side.pieces.into_iter().filter(|piece| context.syncs.takes(kind, &piece.name, piece.repository.as_deref())) {
                    let refused = context.refused(kind, &piece);
                    mine.push(Mine { kind, hash: hash_of(kind, &piece), piece, changed_at: None, refused });
                }
            }
            (mine, unreadable)
        })
        .await
        .map_err(|why| self.kit_not_kept(why))?;
        let (mut mine, unreadable) = read;
        let now = Instant::from(&self.now());
        let mut store = self.store().lock().await;
        for one in &mine {
            store.see_kit_link(&kept_item(&one.key()), &one.hash, now.as_str()).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        }
        let seen = store.kit_link_seen().map_err(|why| self.kit_not_kept(why))?;
        for one in &mut mine {
            one.changed_at = seen.get(&kept_item(&one.key())).map(|(_, at)| Instant::carried(at.clone()));
        }
        Ok((mine, unreadable))
    }

    /// What each linked Mac last offered, `only` one where named, with what this Mac last agreed with it and when
    /// it crossed. An offer this build cannot read is left out until that Mac offers again.
    async fn kit_others(&self, only: Option<&str>) -> Result<Vec<(Other, String)>, Refusal> {
        let store = self.store().lock().await;
        let held = store.kit_offers_held().map_err(|why| self.kit_not_kept(why))?;
        let mut others = Vec::new();
        for kept in held.into_iter().filter(|kept| only.is_none_or(|mac| kept.mac == mac)) {
            let Ok(offer) = ipc::decode::<KitOffer>("a Kit another Mac offered", kept.offer.as_bytes()) else { continue };
            let agreed = store.kit_link_agreed(&kept.mac).map_err(|why| self.kit_not_kept(why))?;
            let agreed = agreed.iter().filter_map(|(item, hash)| Some((key_kept(item)?, hash.clone()))).collect();
            others.push((Other { mac: kept.mac, offer, agreed }, kept.at));
        }
        Ok(others)
    }

    /// `share_kit`: what this Mac sends, cut by `crate::share`.
    pub(crate) async fn kit_offered(&self) -> Result<KitOffer, Refusal> {
        let syncs = self.kit_syncs().await?;
        let context = self.kit_context(syncs.clone()).await?;
        let (mine, _) = self.kit_mine(context.as_ref()).await?;
        Ok(self.kit_for_other_mac(offer(&self.kit_here_name(), &syncs, &mine)).sent())
    }

    /// Take from what `only`, or every linked Mac, offers: put here each piece that changed there only, record the
    /// markers that moved, then tell Bridge and the linked Macs where anything did.
    async fn kit_exchanged(&self, only: Option<&str>) -> Result<(), Refusal> {
        let exchanging = self.home_held().exchanging.lock().await;
        let syncs = self.kit_syncs().await?;
        let context = self.kit_context(syncs.clone()).await?;
        if let Some(context) = &context {
            let (mine, _) = self.kit_mine(Some(context)).await?;
            let others: Vec<Other> = self.kit_others(only).await?.into_iter().map(|(other, _)| other).collect();
            let plan = plan(&syncs, &mine, &others);
            let context = Arc::clone(context);
            let takes = plan.takes.clone();
            let put = tokio::task::spawn_blocking(move || {
                let places = context.places();
                takes.into_iter().map(|take| (pieces::put_here(&places, take.kind, &take.piece, take.replacing.as_ref()), take)).collect::<Vec<_>>()
            })
            .await
            .map_err(|why| self.kit_not_kept(why))?;
            let mut store = self.store().lock().await;
            for (mac, key, hash) in &plan.agrees {
                store.agree_kit_link(mac, &kept_item(key), hash).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
            }
            for (done, take) in put {
                let key = (take.kind, take.piece.repository.clone(), take.piece.name.clone());
                match done {
                    Ok(()) => store.agree_kit_link(&take.mac, &kept_item(&key), &take.hash).map_err(|why| self.kit_not_kept(format!("{why:?}")))?,
                    Err(refused) => {
                        let said = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a Kit item from another Mac was not put here")
                            .with_field("mac", FieldValue::Str(take.mac.clone()))
                            .with_field("item", FieldValue::Str(take.piece.name.clone()))
                            .with_field("why", FieldValue::Str(refused.said()));
                        crate::transcript::note_aloud(&said);
                    }
                }
            }
        }
        drop(exchanging);
        self.kit_link_read().await.map(|_| ())
    }

    /// `hold_other_mac_kit`. **Never refused for want of Kit's folders**, so the link stays up on a Fleet without them.
    pub(crate) async fn kit_held(self: &Arc<Self>, held: OtherMacKit) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&held.mac)?;
        let kept = store::KeptKitOffer { mac: mac.clone(), at: Instant::from(&self.now()).as_str().to_string(), offer: self.encoded(&held.kit)? };
        self.store().lock().await.hold_kit_offer(&kept).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        // A settle queued for it goes first, oldest first, on a task of its own.
        self.project_acts_flushing(&mac).await;
        self.kit_exchanged(Some(&mac)).await?;
        Ok(MacNamed { mac })
    }

    /// `kit_settle_from_mac`: Mac `mac` settled a clash; both now record the side not kept.
    pub(crate) async fn kit_settle_held(&self, mac: String, settled: KitSettled) -> Result<(), Refusal> {
        let mac = self.mac_named(&mac)?;
        let key = key_asked(&settled.item);
        self.store().lock().await.agree_kit_link(&mac, &kept_item(&key), &settled.agreed).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        self.kit_exchanged(Some(&mac)).await
    }

    /// `get_kit_link`, and what every act answers. Tells Bridge and the linked Macs where this Mac's offer or a mark
    /// moved since it last told them.
    pub(crate) async fn kit_link_read(&self) -> Result<KitLink, Refusal> {
        let syncs = self.kit_syncs().await?;
        let context = self.kit_context(syncs.clone()).await?;
        let (mine, unreadable) = self.kit_mine(context.as_ref()).await?;
        let held = self.kit_others(None).await?;
        let others: Vec<Other> = held.iter().map(|(other, _)| other.clone()).collect();
        let here_name = self.kit_here_name();
        let rows = rows(&here_name, &syncs, &mine, &others);
        let paired = self.store().lock().await.paired_macs().unwrap_or_default();
        let mut macs: Vec<KitLinkedMac> = held
            .iter()
            .map(|(other, at)| KitLinkedMac {
                mac: other.mac.clone(),
                name: match other.offer.name.is_empty() {
                    true => paired.iter().find(|one| one.id == other.mac).map_or_else(|| other.mac.clone(), |one| one.name.clone()),
                    false => other.offer.name.clone(),
                },
                crossed_at: Some(Instant::carried(at.clone())),
                kinds: other.offer.kinds.clone(),
                items: other.offer.items.clone(),
            })
            .collect();
        let never: Vec<KitLinkedMac> = paired
            .iter()
            .filter(|one| one.state == store::MacState::Linked && !macs.iter().any(|mac| mac.mac == one.id))
            .map(|linked| KitLinkedMac { mac: linked.id.clone(), name: linked.name.clone(), crossed_at: None, kinds: Vec::new(), items: Vec::new() })
            .collect();
        macs.extend(never);
        // What the linked Macs read, and what Bridge draws: the offer's hashes and every mark.
        let mut told = Sha256::new();
        let offered = offer(&here_name, &syncs, &mine);
        told.update(format!("{:?}{:?}", offered.kinds, offered.items));
        for piece in &offered.pieces {
            told.update(format!("{:?}{:?}{}{}\n", piece.kind, piece.repository, piece.name, piece.hash));
        }
        for row in &rows {
            told.update(format!("{:?}{:?}{}{:?}\n", row.kind, row.repository, row.name, row.against.iter().map(|one| (&one.mac, one.mark)).collect::<Vec<_>>()));
        }
        let told = format!("{:x}", told.finalize());
        let moved = self.home_held().told.lock().unwrap_or_else(PoisonError::into_inner).replace(told.clone()) != Some(told);
        if moved {
            self.publish(Event::KitChanged(KitChanged {}));
        }
        Ok(KitLink {
            kinds: HomeKind::ALL.iter().map(|kind| HomeKindSwitch { kind: *kind, on: syncs.kinds.contains(kind) }).collect(),
            items: syncs.items,
            macs,
            rows,
            unreadable,
        })
    }

    /// `set_kit_link_kind`: switched, then what the linked Macs last offered of it is taken.
    pub(crate) async fn kit_link_kind_switched(&self, switch: SetKitLinkKind) -> Result<KitLink, Refusal> {
        self.store().lock().await.switch_kit_link_kind(switch.kind.as_wire(), switch.on).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        self.kit_exchanged(None).await?;
        self.kit_link_read().await
    }

    /// `set_kit_link_item`.
    pub(crate) async fn kit_link_item_switched(&self, switch: SetKitLinkItem) -> Result<KitLink, Refusal> {
        let key = key_asked(&switch.item);
        self.store().lock().await.switch_kit_link_item(&kept_item(&key), switch.on).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        self.kit_exchanged(None).await?;
        self.kit_link_read().await
    }

    /// `settle_kit_clash`: record the side not kept, tell the other Mac or queue it, and take theirs where it is kept.
    pub(crate) async fn kit_clash_settled(&self, settle: SettleKitClash) -> Result<KitLink, Refusal> {
        let key = key_asked(&settle.item);
        let syncs = self.kit_syncs().await?;
        let context = self.kit_context(syncs).await?;
        let (mine, _) = self.kit_mine(context.as_ref()).await?;
        let Some((other, _)) = self.kit_others(Some(&settle.mac)).await?.into_iter().next() else {
            return Err(Refusal::NotFound(WireError::raised(
                crate::mac_acts::MAC_NOT_LINKED,
                format!("This Mac is not linked to Mac {}.", settle.mac),
                self.run_id(),
            )));
        };
        let here = mine.iter().find(|one| one.key() == key).map(|one| one.hash.clone());
        let there = other.offer.pieces.iter().find(|piece| super::key_of(piece) == key).map(|piece| piece.hash.clone());
        let mark = judged(here.as_deref(), there.as_deref(), other.agreed.get(&key).map(String::as_str)).mark;
        let (Some(here), Some(there), KitMark::Clash) = (here, there, mark) else {
            return Err(Refusal::IllegalMove(WireError::raised(NO_CLASH, format!("{} is not a clash with {}.", key.2, other.offer.name), self.run_id())));
        };
        let agreed = match settle.keep {
            KitKeep::Here => there,
            KitKeep::There => here,
        };
        self.store().lock().await.agree_kit_link(&settle.mac, &kept_item(&key), &agreed).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        let settled = KitSettled { item: item_of(&key), agreed };
        // Behind what is queued already, so the Mac hears them in the order he made them.
        let waiting = self.project_acts_waiting(&settle.mac).await;
        if waiting || matches!(self.kit_settle_sent(&settle.mac, &settled).await, SettleSent::Unreachable) {
            let body = self.encoded(&settled)?;
            let at = self.now_ms();
            self.store().lock().await.queue_kit_settle(&settle.mac, at, &body).map_err(|why| self.kit_not_kept(format!("{why:?}")))?;
        }
        self.kit_exchanged(Some(&settle.mac)).await?;
        self.kit_link_read().await
    }

    /// `settled` handed to Mac `mac` through the link. A refusal there is said aloud and not sent again.
    pub(crate) async fn kit_settle_sent(&self, mac: &str, settled: &KitSettled) -> SettleSent {
        let Ok(body) = self.encoded(settled) else { return SettleSent::Answered };
        let Ok(switch) = self.phone_switch() else { return SettleSent::Unreachable };
        let Ok((status, _)) = switch.send_other_mac(mac, "/other_mac/kit_settle", body.into_bytes()).await else {
            return SettleSent::Unreachable;
        };
        if status == 401 || status >= 500 {
            return SettleSent::Unreachable;
        }
        if !(200..300).contains(&status) {
            let said = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a settled Kit clash was refused by the other Mac, and dropped")
                .with_field("mac", FieldValue::Str(mac.to_string()))
                .with_field("status", FieldValue::Str(status.to_string()));
            crate::transcript::note_aloud(&said);
        }
        SettleSent::Answered
    }
}

/// What the linked Macs last read of this Mac, so a read that changed nothing tells nobody.
pub(crate) type Told = std::sync::Mutex<Option<String>>;
