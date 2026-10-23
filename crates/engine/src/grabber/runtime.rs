use super::{
    crawl::{CrawlError, Fetched, Network},
    store::ProjectStore,
    *,
};
use crate::{events::EventBus, Command, EntryId, JobId, JobState};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;

#[doc(hidden)]
pub enum TransferAction {
    Admit {
        member: ProjectMember,
        request: crate::DownloadRequest,
    },
    Query {
        project: ProjectId,
        generation: u64,
    },
    Hold {
        project: ProjectId,
        generation: u64,
    },
    Retire {
        project: ProjectId,
        generation: u64,
        remove_internal: bool,
    },
    Activate(ProjectMember),
}
#[derive(Clone)]
pub struct TransferStatus {
    pub member: ProjectMember,
    pub job: JobId,
    pub entry: Option<EntryId>,
    pub state: JobState,
    pub held: bool,
    pub received: u64,
    pub destination: PathBuf,
    pub message: String,
}
pub(crate) struct Handle {
    sender: mpsc::Sender<GrabberCommand>,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}
impl Handle {
    pub fn send(&self, command: GrabberCommand) {
        if let Err(error) = self.sender.try_send(command) {
            let command = error.into_inner();
            let _ = command
                .reply
                .try_send(Err("Site Grabber is busy or closing. Try again.".into()));
        }
    }
    pub fn stop(&self) {
        self.stop.cancel();
    }
    pub fn finished(&self) -> bool {
        self.task.is_finished()
    }
}
pub(crate) fn spawn(
    engine: crate::service::WeakEngineClient,
    bus: Arc<EventBus>,
    durable: bool,
) -> Handle {
    let (sender, receiver) = mpsc::channel(32);
    let stop = CancellationToken::new();
    let token = stop.clone();
    let task = tokio::spawn(async move {
        run(engine, bus, receiver, token, durable).await;
    });
    Handle { sender, stop, task }
}
async fn transfer(
    engine: &crate::service::WeakEngineClient,
    action: TransferAction,
) -> Result<Vec<TransferStatus>, String> {
    let (reply, receive) = oneshot::channel();
    engine
        .try_send(Command::ProjectTransfer { action, reply })
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(Duration::from_secs(30), receive)
        .await
        .map_err(|_| {
            "Project admission acknowledgement timed out. Resume to reconcile its durable identity."
                .to_string()
        })?
        .map_err(|_| "The download engine closed before acknowledgement.".to_string())?
}
use super::model::ProjectPending as Pending;
struct Runtime {
    projects: BTreeMap<ProjectId, Project>,
    store: ProjectStore,
    engine: crate::service::WeakEngineClient,
    bus: Arc<EventBus>,
    network: Network,
    crawl: JoinSet<(ProjectId, u64, ItemId, Option<Result<Fetched, CrawlError>>)>,
    inflight: BTreeMap<(ProjectId, ItemId), CancellationToken>,
    exports: JoinSet<(ProjectId, u64, Result<super::export::Published, String>)>,
    export_stops: BTreeMap<ProjectId, CancellationToken>,
    pending: BTreeMap<ProjectId, Pending>,
    resume: BTreeSet<ProjectId>,
    startup_error: Option<String>,
}
async fn run(
    engine: crate::service::WeakEngineClient,
    bus: Arc<EventBus>,
    mut receiver: mpsc::Receiver<GrabberCommand>,
    stop: CancellationToken,
    durable: bool,
) {
    let store = ProjectStore::new(durable);
    let (projects, startup_error) = store.load();
    let network = match Network::new() {
        Ok(n) => n,
        Err(error) => {
            bus.publish(crate::Event::Grabber(GrabberEvent {
                projects: vec![],
                error: Some(error),
            }));
            return;
        }
    };
    let mut runtime = Runtime {
        projects,
        store,
        engine,
        bus,
        network,
        crawl: JoinSet::new(),
        inflight: BTreeMap::new(),
        exports: JoinSet::new(),
        export_stops: BTreeMap::new(),
        pending: BTreeMap::new(),
        resume: BTreeSet::new(),
        startup_error,
    };
    runtime.publish();
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {biased;
            _=stop.cancelled()=>break,
            command=receiver.recv()=>match command{Some(command)=>{
                let result=runtime.command(command.action).await;let _=command.reply.try_send(result);runtime.publish();
            },None=>break},
            result=runtime.crawl.join_next(),if !runtime.crawl.is_empty()=>{
                match result {Some(Ok((project,generation,item,result)))=>runtime.fetched(project,generation,item,result),
                    Some(Err(_))=>runtime.worker_failed(),_=>{}}
                runtime.publish();
            },
            result=runtime.exports.join_next(),if !runtime.exports.is_empty()=>{
                if let Some(Ok((id,generation,result)))=result{
                    runtime.export_stops.remove(&id);
                    if let Some(p)=runtime.projects.get_mut(&id).filter(|p|p.generation==generation){
                        p.downloading=false;
                        match result {Ok(export)=>{p.offline=Some(export.index);p.state=if export.missing==0{ProjectState::Complete}else{ProjectState::Review};p.message=if export.missing==0{"Offline snapshot saved.".into()}else{format!("Snapshot saved with {} missing resources; online references retained.",export.missing)};},
                            Err(error)=>{p.state=ProjectState::Paused;p.message=error;}}
                    }
                    runtime.persist(&id);runtime.publish();
                }else{runtime.startup_error=Some("An offline export worker stopped unexpectedly. Existing snapshots were preserved.".into());}
            },
            _=tick.tick()=>{runtime.reconcile().await;runtime.schedule().await;}
        }
    }
    receiver.close();
    for (_, token) in &runtime.inflight {
        token.cancel();
    }
    for token in runtime.export_stops.values() {
        token.cancel();
    }
    // Join owned workers before signalling engine shutdown completion.
    while let Some(result) = runtime.crawl.join_next().await {
        if let Ok((id, generation, item, result)) = result {
            runtime.fetched(id, generation, item, result);
        }
    }
    while runtime.exports.join_next().await.is_some() {}
    let ids: Vec<_> = runtime.projects.keys().cloned().collect();
    for id in ids {
        if let Some(p) = runtime.projects.get_mut(&id) {
            if matches!(
                p.state,
                ProjectState::Scanning | ProjectState::Downloading | ProjectState::Exporting
            ) {
                p.state = ProjectState::Paused;
                p.message = "Stopped at application shutdown. Resume manually.".into();
            }
        }
        runtime.persist(&id);
    }
}
impl Runtime {
    fn publish(&self) {
        if self.store.root.is_none() {
            return;
        }
        self.bus.publish(crate::Event::Grabber(GrabberEvent {
            projects: self.projects.values().map(Project::snapshot).collect(),
            error: self.startup_error.clone(),
        }));
    }
    fn persist(&mut self, id: &ProjectId) -> bool {
        let Some(p) = self.projects.get(id) else {
            return false;
        };
        if let Err(error) = self.store.save(p) {
            if let Some(p) = self.projects.get_mut(id) {
                p.state = ProjectState::Paused;
                p.message = format!("Project storage failed. {error}");
            }
            for ((project, _), token) in &self.inflight {
                if project == id {
                    token.cancel();
                }
            }
            if let Some(token) = self.export_stops.get(id) {
                token.cancel();
            }
            false
        } else {
            true
        }
    }
    fn query(&self, id: Option<&ProjectId>, offset: usize, filter: &str) -> GrabberReply {
        let p = id.and_then(|id| self.projects.get(id));
        let filter = filter.to_ascii_lowercase();
        let matched: Vec<_> = p
            .into_iter()
            .flat_map(|p| p.items.values())
            .filter(|i| {
                filter.is_empty()
                    || i.url.to_ascii_lowercase().contains(&filter)
                    || i.mime.contains(&filter)
            })
            .collect();
        let offset = if matched.is_empty() {
            0
        } else {
            offset.min((matched.len() - 1) / 200 * 200)
        };
        GrabberReply {
            projects: self.projects.values().map(Project::snapshot).collect(),
            project: p.map(Project::snapshot),
            config: p.map(|p| p.config.clone()),
            results: matched
                .iter()
                .skip(offset)
                .take(200)
                .map(|i| (*i).clone())
                .collect(),
            offset,
            total: matched.len(),
        }
    }
    fn idle(&self, id: &ProjectId) -> Result<(), String> {
        let p = self.projects.get(id).ok_or("Project no longer exists.")?;
        if self.inflight.keys().any(|(project, _)| project == id)
            || self.export_stops.contains_key(id)
            || matches!(
                p.state,
                ProjectState::Scanning | ProjectState::Downloading | ProjectState::Exporting
            )
            || self.pending.contains_key(id)
            || p.pending.is_some()
        {
            return Err(
                "Stop this project and wait for its current work before changing it.".into(),
            );
        }
        Ok(())
    }
    async fn command(&mut self, action: GrabberAction) -> Result<GrabberReply, String> {
        let id = match action {
            GrabberAction::Query {
                project,
                offset,
                filter,
            } => return Ok(self.query(project.as_ref(), offset, &filter)),
            GrabberAction::Save {
                project,
                mut config,
                rescan,
            } => {
                config.validate()?;
                if let Some(id) = project {
                    if rescan {
                        if !self.projects.contains_key(&id) {
                            return Err("Project no longer exists.".into());
                        }
                        if self.pending.contains_key(&id) {
                            return Err("Project retirement is still in progress.".into());
                        }
                        for ((project, _), token) in &self.inflight {
                            if project == &id {
                                token.cancel();
                            }
                        }
                        if let Some(token) = self.export_stops.get(&id) {
                            token.cancel();
                        }
                        self.pending.insert(id.clone(), Pending::Rescan(config));
                        self.retire(&id).await?;
                    } else {
                        self.idle(&id)?;
                        let p = self.projects.get(&id).ok_or("Project no longer exists.")?;
                        if p.config == config {
                            return Ok(self.query(Some(&id), 0, ""));
                        }
                        if !p.items.is_empty() && p.state != ProjectState::Idle {
                            return Err("Configuration changed. Use Rescan to start a fresh scan generation.".into());
                        }
                        let p = self.projects.get_mut(&id).expect("project");
                        p.config = config;
                        p.items.clear();
                        p.frontier.clear();
                        p.next_item = 1;
                        seed(p);
                    }
                    id
                } else {
                    if self.projects.len() >= 128 {
                        return Err(
                            "At most 128 saved projects. Delete an unused project first.".into(),
                        );
                    }
                    let id = super::store::new_id()?;
                    let mut p = Project {
                        version: 1,
                        id: id.clone(),
                        generation: 1,
                        config,
                        state: ProjectState::Idle,
                        items: BTreeMap::new(),
                        frontier: VecDeque::new(),
                        next_item: 1,
                        cache_bytes: 0,
                        message: String::new(),
                        publication: 0,
                        offline: None,
                        downloading: false,
                        limit_reached: false,
                        pending: None,
                    };
                    seed(&mut p);
                    self.store.save(&p)?;
                    self.projects.insert(id.clone(), p);
                    id
                }
            }
            GrabberAction::Scan(id) | GrabberAction::Resume(id) => {
                if self.pending.contains_key(&id) {
                    return Err("Project retirement is still in progress.".into());
                }
                if let Some(pending) = self.projects.get(&id).and_then(|p| p.pending.clone()) {
                    self.pending.insert(id.clone(), pending);
                    self.retire(&id).await?;
                    return Ok(self.query(Some(&id), 0, ""));
                }
                self.network.clear_failures().await;
                let p = self
                    .projects
                    .get_mut(&id)
                    .ok_or("Project no longer exists.")?;
                if p.state == ProjectState::Exporting {
                    return Err("Wait for export to finish.".into());
                }
                if p.limit_reached {
                    return Err("A project limit was reached. Rescan with higher limits or narrower filters.".into());
                }
                self.resume.insert(id.clone());
                p.message.clear();
                p.state = if !p.frontier.is_empty() {
                    ProjectState::Scanning
                } else if p.downloading {
                    ProjectState::Downloading
                } else {
                    ProjectState::Review
                };
                id
            }
            GrabberAction::Stop(id) => {
                self.resume.remove(&id);
                for ((project, _), token) in &self.inflight {
                    if project == &id {
                        token.cancel();
                    }
                }
                if let Some(token) = self.export_stops.get(&id) {
                    token.cancel();
                }
                let p = self
                    .projects
                    .get_mut(&id)
                    .ok_or("Project no longer exists.")?;
                p.state = ProjectState::Paused;
                p.message = "Stopped. Results and progress retained.".into();
                transfer(
                    &self.engine,
                    TransferAction::Hold {
                        project: id.clone(),
                        generation: p.generation,
                    },
                )
                .await?;
                id
            }
            GrabberAction::Download(id) => {
                self.idle(&id)?;
                let p = self
                    .projects
                    .get_mut(&id)
                    .ok_or("Project no longer exists.")?;
                if !p.frontier.is_empty() && !p.limit_reached {
                    return Err("Finish the scan before downloading selected results.".into());
                }
                for item in p
                    .items
                    .values_mut()
                    .filter(|i| matches!(i.status, ItemStatus::Discovered | ItemStatus::Fetching))
                {
                    item.eligible = false;
                    item.selected = false;
                }
                if p.config.mode == GrabberMode::OfflineWebsite {
                    if let Some(seed) = p.items.get_mut(&ItemId(1)) {
                        seed.selected = true;
                    }
                }
                select_dependencies(p);
                if !p.items.values().any(|i| i.selected && i.eligible) {
                    return Err("Select at least one eligible result.".into());
                }
                self.resume.insert(id.clone());
                p.downloading = true;
                p.state = ProjectState::Downloading;
                p.message = "Downloading selected resources.".into();
                id
            }
            GrabberAction::RetryFailed(id) => {
                self.idle(&id)?;
                let p = self
                    .projects
                    .get_mut(&id)
                    .ok_or("Project no longer exists.")?;
                if p.items
                    .values()
                    .any(|i| i.original.is_some() && i.status == ItemStatus::Missing)
                {
                    return Err("A frozen scan original is missing or changed. Use Rescan to create a fresh generation.".into());
                }
                for item in p
                    .items
                    .values_mut()
                    .filter(|i| matches!(i.status, ItemStatus::Failed | ItemStatus::Missing))
                {
                    if item.entry.is_some() {
                        item.local_identity = None;
                        item.status = ItemStatus::Ready;
                    } else {
                        item.status = ItemStatus::Discovered;
                        if !p.frontier.contains(&item.id) {
                            p.frontier.push_back(item.id);
                        }
                    }
                    item.message.clear();
                }
                p.state = if p.frontier.is_empty() {
                    ProjectState::Downloading
                } else {
                    ProjectState::Scanning
                };
                p.downloading = p.frontier.is_empty();
                let members: Vec<_> = p
                    .items
                    .values()
                    .filter(|i| {
                        i.selected
                            && i.eligible
                            && i.entry.is_some()
                            && i.status == ItemStatus::Ready
                    })
                    .map(|i| p.member(i.id))
                    .collect();
                for member in members {
                    transfer(&self.engine, TransferAction::Activate(member)).await?;
                }
                self.network.clear_failures().await;
                id
            }
            GrabberAction::Select {
                project,
                items,
                selected,
            } => {
                self.idle(&project)?;
                let p = self
                    .projects
                    .get_mut(&project)
                    .ok_or("Project no longer exists.")?;
                for id in items.into_iter().take(200) {
                    if let Some(i) = p.items.get_mut(&id) {
                        if i.eligible
                            && !(id == ItemId(1) && p.config.mode == GrabberMode::OfflineWebsite)
                        {
                            i.selected = selected;
                            i.explicit_selection = true;
                        }
                    }
                }
                select_dependencies(p);
                project
            }
            GrabberAction::Delete(id) => {
                if self.pending.contains_key(&id) {
                    return Err("Project retirement is still in progress.".into());
                }
                for ((project, _), token) in &self.inflight {
                    if project == &id {
                        token.cancel();
                    }
                }
                if let Some(token) = self.export_stops.get(&id) {
                    token.cancel();
                }
                let p = self
                    .projects
                    .get_mut(&id)
                    .ok_or("Project no longer exists.")?;
                p.state = ProjectState::Paused;
                self.pending.insert(id.clone(), Pending::Delete);
                self.retire(&id).await?;
                id
            }
        };
        if self.projects.contains_key(&id) && !self.persist(&id) {
            return Err(self.projects[&id].message.clone());
        }
        Ok(self.query(Some(&id), 0, ""))
    }
    async fn retire(&mut self, id: &ProjectId) -> Result<(), String> {
        let generation = self
            .projects
            .get(id)
            .ok_or("Project no longer exists.")?
            .generation;
        if let Some(p) = self.projects.get_mut(id) {
            p.pending = self.pending.get(id).cloned();
            p.state = ProjectState::Paused;
        }
        if !self.persist(id) {
            self.pending.remove(id);
            return Err(
                "Cannot save the pending project operation. Resume after storage is available."
                    .into(),
            );
        }
        transfer(
            &self.engine,
            TransferAction::Retire {
                project: id.clone(),
                generation: 0,
                remove_internal: true,
            },
        )
        .await?;
        if let Some(p) = self.projects.get_mut(id) {
            p.state = ProjectState::Paused;
            p.message =
                format!("Waiting for scan {generation} workers and owned downloads to stop.");
        }
        Ok(())
    }
    fn worker_failed(&mut self) {
        let ids: Vec<_> = self.inflight.keys().map(|(id, _)| id.clone()).collect();
        for id in ids {
            if let Some(p) = self.projects.get_mut(&id) {
                p.state = ProjectState::Paused;
                p.message =
                    "A crawler worker stopped unexpectedly. Resume to retry interrupted resources."
                        .into();
                for item in p
                    .items
                    .values_mut()
                    .filter(|i| i.status == ItemStatus::Fetching)
                {
                    item.status = ItemStatus::Discovered;
                    if !p.frontier.contains(&item.id) {
                        p.frontier.push_back(item.id);
                    }
                }
            }
            self.persist(&id);
        }
        for token in self.inflight.values() {
            token.cancel();
        }
        self.inflight.clear();
    }
    fn fetched(
        &mut self,
        id: ProjectId,
        generation: u64,
        item_id: ItemId,
        result: Option<Result<Fetched, CrawlError>>,
    ) {
        self.inflight.remove(&(id.clone(), item_id));
        let Some(p) = self
            .projects
            .get_mut(&id)
            .filter(|p| p.generation == generation)
        else {
            return;
        };
        let Some(mut item) = p.items.remove(&item_id) else {
            return;
        };
        match result {
            None => {
                item.status = ItemStatus::Discovered;
                if !p.frontier.contains(&item_id) {
                    p.frontier.push_back(item_id);
                }
            }
            Some(Err(error)) => {
                item.message = error.message.clone();
                if error.blocked || error.limit {
                    item.status = ItemStatus::Discovered;
                    p.frontier.push_back(item_id);
                    if error.limit {
                        p.limit_reached = true;
                        p.state = ProjectState::Paused;
                        p.message = error.message;
                    }
                } else {
                    item.status = ItemStatus::Failed;
                }
            }
            Some(Ok(fetched)) => {
                item.final_url = fetched.url;
                item.mime = fetched.mime;
                item.size = fetched.size;
                if item.kind == ItemKind::Page && fetched.kind != ItemKind::Page {
                    item.depth = item.depth.saturating_sub(1);
                }
                item.kind = fetched.kind;
                let outside_depth = item.kind == ItemKind::Page && item.depth > p.config.max_depth;
                let url = public_url(&item.final_url).expect("validated fetched URL");
                item.eligible = if item.kind == ItemKind::Page {
                    (p.config.mode == GrabberMode::OfflineWebsite
                        && (item.id == ItemId(1)
                            || p.config.include_urls.is_empty()
                            || p.config
                                .include_urls
                                .iter()
                                .any(|pattern| super::model::glob(pattern, url.as_str()))))
                        || ((!p.config.extensions.is_empty() || !p.config.mime_types.is_empty())
                            && p.config.matches_file(&url, &item.mime))
                } else {
                    p.config.matches_file(&url, &item.mime)
                };
                if outside_depth {
                    item.eligible = false;
                }
                if !item.explicit_selection {
                    item.selected = item.eligible;
                }
                item.status = if !outside_depth && (item.eligible || item.kind == ItemKind::Page) {
                    ItemStatus::Ready
                } else {
                    ItemStatus::Excluded
                };
                item.message = if outside_depth {
                    "Page exceeds the navigation depth limit; its body was not fetched.".into()
                } else if item.mime.is_empty() && !p.config.mime_types.is_empty() {
                    "MIME type unavailable; this result cannot satisfy the MIME filter.".into()
                } else {
                    String::new()
                };
                if let Some(original) = fetched.original {
                    if p.cache_bytes + original.len() as u64 > p.config.cache_limit {
                        p.limit_reached = true;
                        item.status = ItemStatus::Discovered;
                        p.frontier.push_front(item_id);
                        p.state = ProjectState::Paused;
                        p.message="Scan cache limit reached. Narrow the scope or increase the cache limit and rescan.".into();
                    } else {
                        let saved=self.store.raw(p,item_id).and_then(|path|{
                            // Existing scan originals are immutable; an interrupted complete write is reusable.
                            if path.exists(){super::store::reject_link(&path)?;let old=super::store::read_document(&path,p.config.document_limit)?;if old!=original{return Err("An immutable cached original changed. Rescan this project.".into());}Ok(())}
                            else{super::store::write_new(&path,&original)}
                        });
                        match saved {
                            Ok(()) => {
                                p.cache_bytes += original.len() as u64;
                                item.original = Some(format!("{}.raw", item_id.0));
                                item.original_hash = Some(super::store::hash(&original));
                            }
                            Err(error) => {
                                item.status = ItemStatus::Missing;
                                p.state = ProjectState::Paused;
                                p.message = error;
                            }
                        }
                    }
                }
                if item.status != ItemStatus::Discovered && item.status != ItemStatus::Missing {
                    for link in fetched.links {
                        let depth = if link.navigation {
                            item.depth.saturating_add(1)
                        } else {
                            item.depth
                        };
                        // Probe headers first: an anchor can target a file rather than a page.
                        // The crawler will not fetch an HTML body beyond max_depth.
                        let Ok(url) = public_url(&link.url) else {
                            continue;
                        };
                        if !(p.config.allows(&url, link.navigation)
                            || (!link.navigation && p.config.allows(&url, false))
                            || (link.navigation && p.config.allows(&url, false)))
                        {
                            continue;
                        }
                        let existing = if url.as_str() == item.url || url.as_str() == item.final_url
                        {
                            Some(item_id)
                        } else {
                            p.items
                                .values()
                                .find(|i| i.url == url.as_str() || i.final_url == url.as_str())
                                .map(|i| i.id)
                        };
                        let dependency = if let Some(existing) = existing {
                            existing
                        } else {
                            if p.items.len() + 1 >= p.config.max_results {
                                p.limit_reached = true;
                                p.state = ProjectState::Paused;
                                p.message="Result limit reached. Increase the limit or narrow the filters and rescan.".into();
                                break;
                            }
                            let next = ItemId(p.next_item);
                            p.next_item += 1;
                            p.items.insert(
                                next,
                                new_item(
                                    next,
                                    url.into(),
                                    depth,
                                    if link.navigation {
                                        ItemKind::Page
                                    } else {
                                        link.kind
                                    },
                                ),
                            );
                            p.frontier.push_back(next);
                            next
                        };
                        if dependency != item_id && !item.dependencies.contains(&dependency) {
                            item.dependencies.push(dependency);
                        }
                    }
                }
            }
        }
        p.items.insert(item_id, item);
        self.persist(&id);
    }
    async fn schedule(&mut self) {
        if self.inflight.len() >= 2 {
            return;
        }
        let ids: Vec<_> = self.projects.keys().cloned().collect();
        for id in ids {
            if self.inflight.len() >= 2 {
                break;
            }
            let Some(p) = self.projects.get(&id) else {
                continue;
            };
            if p.state != ProjectState::Scanning || self.pending.contains_key(&id) {
                continue;
            }
            let running = self
                .inflight
                .keys()
                .filter(|(project, _)| project == &id)
                .count();
            if running >= p.config.scan_workers {
                continue;
            }
            let mut candidates: Vec<_> = p.frontier.iter().copied().collect();
            candidates.sort_by_key(|id| p.items.get(id).map(|i| i.depth).unwrap_or(u16::MAX));
            let active_depth = self
                .inflight
                .keys()
                .filter(|(project, _)| project == &id)
                .filter_map(|(_, item)| p.items.get(item).map(|i| i.depth))
                .min();
            let mut candidate = None;
            for next in candidates {
                if let Some(item) = p.items.get(&next) {
                    if active_depth.is_some_and(|depth| item.depth > depth) {
                        continue;
                    }
                    if !self.network.blocked(&item.url).await {
                        candidate = Some(next);
                        break;
                    }
                }
            }
            let Some(next) = candidate else {
                if running == 0 {
                    let p = self.projects.get_mut(&id).expect("project");
                    p.state = if p.frontier.is_empty() {
                        ProjectState::Review
                    } else {
                        ProjectState::Paused
                    };
                    p.message = if p.frontier.is_empty() {
                        "Scan complete. Review selections before downloading.".into()
                    } else {
                        "Some origins have unavailable robots rules. Resume to retry.".into()
                    };
                    self.persist(&id);
                    self.publish();
                }
                continue;
            };
            let p = self.projects.get_mut(&id).expect("project");
            let pages = p
                .items
                .values()
                .filter(|i| i.kind == ItemKind::Page && i.original.is_some())
                .count()
                + running;
            if pages >= p.config.max_pages
                && p.items.get(&next).is_some_and(|i| i.kind == ItemKind::Page)
            {
                p.limit_reached = true;
                p.state = ProjectState::Paused;
                p.message =
                    "Page limit reached. Increase the limit or narrow the scope and rescan.".into();
                self.persist(&id);
                self.publish();
                continue;
            }
            p.frontier.retain(|i| *i != next);
            let item = p.items.get_mut(&next).expect("frontier item");
            item.status = ItemStatus::Fetching;
            let item = item.clone();
            let generation = p.generation;
            let config = p.config.clone();
            if !self.persist(&id) {
                continue;
            }
            let network = self.network.clone();
            let token = CancellationToken::new();
            self.inflight.insert((id.clone(), next), token.clone());
            self.crawl.spawn(async move{let result=tokio::select!{biased;_=token.cancelled()=>None,result=network.fetch(item,config)=>Some(result)};(id,generation,next,result)});
        }
    }
    async fn reconcile(&mut self) {
        let ids: Vec<_> = self.projects.keys().cloned().collect();
        for id in ids {
            if self.pending.contains_key(&id) {
                let result = transfer(
                    &self.engine,
                    TransferAction::Retire {
                        project: id.clone(),
                        generation: 0,
                        remove_internal: true,
                    },
                )
                .await;
                match result {
                    Ok(states)
                        if states.iter().all(|s| s.state.is_terminal())
                            && !self.inflight.keys().any(|(p, _)| p == &id)
                            && !self.export_stops.contains_key(&id) =>
                    {
                        match self.pending.remove(&id).expect("pending operation") {
                            Pending::Delete => match self.store.remove(&id) {
                                Ok(()) => {
                                    self.projects.remove(&id);
                                }
                                Err(error) => {
                                    if let Some(p) = self.projects.get_mut(&id) {
                                        p.message = error;
                                    }
                                }
                            },
                            Pending::Rescan(config) => {
                                let p = self.projects.get_mut(&id).expect("project");
                                p.generation += 1;
                                p.pending = None;
                                p.config = config;
                                p.items.clear();
                                p.frontier.clear();
                                p.next_item = 1;
                                p.cache_bytes = 0;
                                p.downloading = false;
                                p.limit_reached = false;
                                p.state = ProjectState::Scanning;
                                p.message.clear();
                                seed(p);
                                self.persist(&id);
                            }
                        }
                        self.publish();
                    }
                    Err(error) => {
                        self.pending.remove(&id);
                        if let Some(p) = self.projects.get_mut(&id) {
                            p.message = error;
                            p.state = ProjectState::Paused;
                        }
                        self.persist(&id);
                        self.publish();
                    }
                    _ => {}
                }
                continue;
            }
            let Some(p) = self.projects.get(&id) else {
                continue;
            };
            if p.state != ProjectState::Downloading {
                continue;
            }
            let generation = p.generation;
            let states = match transfer(
                &self.engine,
                TransferAction::Query {
                    project: id.clone(),
                    generation,
                },
            )
            .await
            {
                Ok(s) => s,
                Err(error) => {
                    self.fail(&id, error);
                    continue;
                }
            };
            if self.resume.remove(&id) {
                let members: Vec<_> = states
                    .iter()
                    .filter(|state| {
                        state.held
                            && self.projects[&id]
                                .items
                                .get(&state.member.item)
                                .is_some_and(|i| i.selected && i.eligible)
                    })
                    .map(|s| s.member.clone())
                    .collect();
                for member in members {
                    if let Err(error) =
                        transfer(&self.engine, TransferAction::Activate(member)).await
                    {
                        self.fail(&id, error);
                        break;
                    }
                }
                if self.projects[&id].state == ProjectState::Paused {
                    continue;
                }
            }
            let mut known = BTreeSet::new();
            {
                let p = self.projects.get_mut(&id).expect("project");
                for state in &states {
                    if let Some(item) = p.items.get_mut(&state.member.item) {
                        known.insert(item.id);
                        item.entry = state.entry;
                        item.local = Some(state.destination.clone());
                        item.status = match state.state {
                            JobState::Completed => {
                                let identity = crate::inspect_destination(&state.destination)
                                    .ok()
                                    .flatten();
                                if identity.is_some()
                                    && item
                                        .local_identity
                                        .as_ref()
                                        .is_none_or(|old| Some(old) == identity.as_ref())
                                {
                                    item.local_identity = identity;
                                    ItemStatus::Complete
                                } else {
                                    ItemStatus::Missing
                                }
                            }
                            JobState::Failed | JobState::Cancelled => ItemStatus::Failed,
                            JobState::Running | JobState::Pausing | JobState::Cancelling => {
                                ItemStatus::Downloading
                            }
                            _ => ItemStatus::Queued,
                        };
                        item.message = state.message.clone();
                    }
                }
            }
            let mut admissions = 0;
            let items: Vec<_> = self.projects[&id]
                .items
                .values()
                .filter(|i| i.selected && i.eligible)
                .map(|i| i.id)
                .collect();
            for item_id in items {
                let p = &self.projects[&id];
                let item = &p.items[&item_id];
                if p.config.mode == GrabberMode::OfflineWebsite && item.original.is_some() {
                    self.projects
                        .get_mut(&id)
                        .unwrap()
                        .items
                        .get_mut(&item_id)
                        .unwrap()
                        .status = ItemStatus::Complete;
                    continue;
                }
                if !known.contains(&item_id) && item.status == ItemStatus::Complete {
                    let identity = item
                        .local
                        .as_ref()
                        .and_then(|path| crate::inspect_destination(path).ok().flatten());
                    if identity.is_some() && identity == item.local_identity {
                        continue;
                    }
                    let item = self
                        .projects
                        .get_mut(&id)
                        .unwrap()
                        .items
                        .get_mut(&item_id)
                        .unwrap();
                    item.status = ItemStatus::Missing;
                    item.message = "Downloaded file is missing or changed. Retry explicitly; existing files are preserved.".into();
                    continue;
                }
                if known.contains(&item_id) {
                    continue;
                }
                if matches!(
                    item.status,
                    ItemStatus::Failed | ItemStatus::Missing | ItemStatus::Excluded
                ) {
                    continue;
                }
                let destination = if p.config.mode == GrabberMode::OfflineWebsite {
                    match self.store.directory(&id) {
                        Ok(dir) => dir
                            .join(format!("scan-{}", p.generation))
                            .join("resources")
                            .join(super::export::filename(item)),
                        Err(error) => {
                            self.fail(&id, error);
                            break;
                        }
                    }
                } else {
                    p.config
                        .destination
                        .join(format!("rusty-files-{}", p.id.0))
                        .join(format!("scan-{}", p.generation))
                        .join(super::export::filename(item))
                };
                if let Err(error) =
                    super::store::guarded_directory(destination.parent().expect("resource parent"))
                {
                    self.fail(&id, error);
                    break;
                }
                let mut request = match crate::validate_request(
                    &item.final_url,
                    destination,
                    crate::ReplacementPolicy::Deny,
                ) {
                    Ok(r) => r,
                    Err(error) => {
                        self.fail(&id, error.message);
                        break;
                    }
                };
                request.description = format!("Site Grabber: {}", p.config.name);
                request.options = p.config.transfer_options.clone();
                if p.config.mode == GrabberMode::OfflineWebsite {
                    request.options.temporary_directory = None;
                }
                let member = p.member(item_id);
                // Persist the intent before admission; the engine's durable identity wins after a lost reply.
                if !self.persist(&id) {
                    break;
                }
                match transfer(&self.engine, TransferAction::Admit { member, request }).await {
                    Ok(states) => {
                        if let Some(state) = states.into_iter().find(|s| s.member.item == item_id) {
                            let item = self
                                .projects
                                .get_mut(&id)
                                .unwrap()
                                .items
                                .get_mut(&item_id)
                                .unwrap();
                            item.entry = state.entry;
                            item.local = Some(state.destination);
                            item.status = ItemStatus::Queued;
                        }
                    }
                    Err(error) => {
                        self.fail(&id, error);
                        break;
                    }
                }
                admissions += 1;
                if admissions >= 2 {
                    break;
                }
            }
            if !self.persist(&id) || self.projects[&id].state == ProjectState::Paused {
                let _ = transfer(
                    &self.engine,
                    TransferAction::Hold {
                        project: id.clone(),
                        generation,
                    },
                )
                .await;
                continue;
            }
            let p = &self.projects[&id];
            let done = p.state == ProjectState::Downloading
                && p.items
                    .values()
                    .filter(|i| i.selected && i.eligible)
                    .all(|i| {
                        matches!(
                            i.status,
                            ItemStatus::Complete | ItemStatus::Failed | ItemStatus::Missing
                        )
                    });
            if done {
                if p.config.mode == GrabberMode::OfflineWebsite {
                    let mut copy = p.clone();
                    copy.publication += 1;
                    let store = ProjectStore {
                        root: self.store.root.clone(),
                    };
                    let token = CancellationToken::new();
                    let worker_token = token.clone();
                    let worker_id = id.clone();
                    let p = self.projects.get_mut(&id).unwrap();
                    p.publication = copy.publication;
                    p.state = ProjectState::Exporting;
                    if self.persist(&id) {
                        self.export_stops.insert(id.clone(), token);
                        self.exports.spawn_blocking(move || {
                            let result = super::export::publish(&copy, &store, &worker_token);
                            (worker_id, generation, result)
                        });
                    }
                } else {
                    let p = self.projects.get_mut(&id).unwrap();
                    p.downloading = false;
                    p.state = if p.items.values().any(|i| {
                        i.selected && matches!(i.status, ItemStatus::Failed | ItemStatus::Missing)
                    }) {
                        ProjectState::Review
                    } else {
                        ProjectState::Complete
                    };
                    p.message =
                        "Selected file transfers finished. Review any failed or missing files."
                            .into();
                    self.persist(&id);
                }
            }
        }
        self.publish();
    }
    fn fail(&mut self, id: &ProjectId, error: String) {
        if let Some(p) = self.projects.get_mut(id) {
            p.state = ProjectState::Paused;
            p.message = error;
        }
        self.persist(id);
    }
}
fn new_item(id: ItemId, url: String, depth: u16, kind: ItemKind) -> GrabberItem {
    GrabberItem {
        id,
        url: url.clone(),
        final_url: url,
        kind,
        depth,
        selected: false,
        eligible: false,
        explicit_selection: false,
        mime: String::new(),
        size: None,
        status: ItemStatus::Discovered,
        message: String::new(),
        dependencies: vec![],
        original: None,
        original_hash: None,
        entry: None,
        local: None,
        local_identity: None,
    }
}
fn seed(p: &mut Project) {
    let id = ItemId(p.next_item);
    p.next_item += 1;
    p.items.insert(
        id,
        new_item(id, p.config.start_url.clone(), 0, ItemKind::Page),
    );
    p.frontier.push_back(id);
}
fn select_dependencies(p: &mut Project) {
    if p.config.mode != GrabberMode::OfflineWebsite {
        return;
    }
    let mut todo: Vec<_> = p
        .items
        .values()
        .filter(|i| i.selected && i.kind == ItemKind::Page)
        .map(|i| i.id)
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(id) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        let dependencies = p
            .items
            .get(&id)
            .map(|i| i.dependencies.clone())
            .unwrap_or_default();
        for next in dependencies {
            if let Some(item) = p.items.get_mut(&next) {
                if item.kind != ItemKind::Page
                    && item.eligible
                    && (!item.explicit_selection || item.selected)
                {
                    item.selected = true;
                    todo.push(next);
                }
            }
        }
    }
}
