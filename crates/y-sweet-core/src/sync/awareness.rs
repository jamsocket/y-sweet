use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;
use yrs::block::ClientID;
use yrs::updates::decoder::{Decode, Decoder};
use yrs::updates::encoder::{Encode, Encoder};
use yrs::{Doc, Observer, Subscription};

const NULL_STR: &str = "null";

/// How long the metadata (logical clock) of a client is retained after its
/// state has been removed. Within this window, stale updates from the
/// disconnected client are still rejected by the clock check in
/// [Awareness::apply_update]. Matches the `outdatedTimeout` convention of the
/// reference y-protocols implementation (30s).
///
/// Without pruning, a long-lived server-side [Awareness] instance grows by one
/// `meta` entry for every client that ever connected to the document.
const DISCONNECTED_META_TTL: time::Duration = time::Duration::seconds(30);

#[cfg(not(feature = "sync"))]
type AwarenessObserver = Observer<Arc<dyn Fn(&Awareness, &Event) + 'static>>;
#[cfg(feature = "sync")]
type AwarenessObserver = Observer<Arc<dyn Fn(&Awareness, &Event) + Send + Sync + 'static>>;

/// The Awareness class implements a simple shared state protocol that can be used for non-persistent
/// data like awareness information (cursor, username, status, ..). Each client can update its own
/// local state and listen to state changes of remote clients.
///
/// Each client is identified by a unique client id (something we borrow from `doc.clientID`).
/// A client can override its own state by propagating a message with an increasing timestamp
/// (`clock`). If such a message is received, it is applied if the known state of that client is
/// older than the new state (`clock < new_clock`). If a client thinks that a remote client is
/// offline, it may propagate a message with `{ clock, state: null, client }`. If such a message is
/// received, and the known clock of that client equals the received clock, it will clean the state.
///
/// Before a client disconnects, it should propagate a `null` state with an updated clock.
pub struct Awareness {
    pub doc: Doc,
    states: HashMap<ClientID, String>,
    meta: HashMap<ClientID, MetaClientState>,
    on_update: Option<AwarenessObserver>,
}

impl Awareness {
    /// Creates a new instance of [Awareness] struct, which operates over a given document.
    /// Awareness instance has full ownership of that document. If necessary it can be accessed
    /// using either [Awareness::doc] or [Awareness::doc_mut] methods.
    pub fn new(doc: Doc) -> Self {
        Awareness {
            doc,
            on_update: None,
            states: HashMap::new(),
            meta: HashMap::new(),
        }
    }

    /// Returns a channel receiver for an incoming awareness events. This channel can be cloned.
    #[cfg(feature = "sync")]
    pub fn on_update<F>(&mut self, f: F) -> Subscription
    where
        F: Fn(&Awareness, &Event) + Send + Sync + 'static,
    {
        let eh = self.on_update.get_or_insert_with(Observer::default);
        eh.subscribe(Arc::new(f))
    }

    /// Returns a channel receiver for an incoming awareness events. This channel can be cloned.
    #[cfg(not(feature = "sync"))]
    pub fn on_update<F>(&mut self, f: F) -> Subscription
    where
        F: Fn(&Awareness, &Event) + 'static,
    {
        let eh = self.on_update.get_or_insert_with(Observer::default);
        eh.subscribe(Arc::new(f))
    }

    /// Returns a read-only reference to an underlying [Doc].
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Returns a read-write reference to an underlying [Doc].
    pub fn doc_mut(&mut self) -> &mut Doc {
        &mut self.doc
    }

    /// Returns a globally unique client ID of an underlying [Doc].
    pub fn client_id(&self) -> ClientID {
        self.doc.client_id()
    }

    /// Returns a state map of all of the clients tracked by current [Awareness] instance. Those
    /// states are identified by their corresponding [ClientID]s. The associated state is
    /// represented and replicated to other clients as a JSON string.
    pub fn clients(&self) -> &HashMap<ClientID, String> {
        &self.states
    }

    /// Returns a JSON string state representation of a current [Awareness] instance.
    pub fn local_state(&self) -> Option<&str> {
        Some(self.states.get(&self.doc.client_id())?.as_str())
    }

    /// Sets a current [Awareness] instance state to a corresponding JSON string. This state will
    /// be replicated to other clients as part of the [AwarenessUpdate] and it will trigger an event
    /// to be emitted if current instance was created using Awareness::with_observer method.
    ///
    pub fn set_local_state<S: Into<String>>(&mut self, json: S) {
        let client_id = self.doc.client_id();
        self.update_meta(client_id);
        let new: String = json.into();
        match self.states.entry(client_id) {
            Entry::Occupied(mut e) => {
                e.insert(new);
                if let Some(eh) = self.on_update.as_ref() {
                    let e = Event::new(vec![], vec![client_id], vec![]);
                    eh.trigger(|cb| {
                        cb(self, &e);
                    });
                }
            }
            Entry::Vacant(e) => {
                e.insert(new);
                if let Some(eh) = self.on_update.as_ref() {
                    let e = Event::new(vec![client_id], vec![], vec![]);
                    eh.trigger(|cb| {
                        cb(self, &e);
                    });
                }
            }
        }
    }

    /// Clears out a state of a given client, effectively marking it as disconnected.
    pub fn remove_state(&mut self, client_id: ClientID) {
        let prev_state = self.states.remove(&client_id);
        self.update_meta(client_id);
        self.prune_disconnected_meta(DISCONNECTED_META_TTL);
        if let Some(eh) = self.on_update.as_ref() {
            if prev_state.is_some() {
                let e = Event::new(Vec::default(), Vec::default(), vec![client_id]);
                eh.trigger(|cb| {
                    cb(self, &e);
                });
            }
        }
    }

    /// Clears out a state of a current client (see: [Awareness::client_id]),
    /// effectively marking it as disconnected.
    pub fn clean_local_state(&mut self) {
        let client_id = self.doc.client_id();
        self.remove_state(client_id);
    }

    fn update_meta(&mut self, client_id: ClientID) {
        match self.meta.entry(client_id) {
            Entry::Occupied(mut e) => {
                let clock = e.get().clock + 1;
                let meta = MetaClientState::new(clock);
                e.insert(meta);
            }
            Entry::Vacant(e) => {
                e.insert(MetaClientState::new(1));
            }
        }
    }

    /// Drop metadata entries of clients that no longer have a state and whose
    /// metadata hasn't been touched for `ttl`. The entry of a client that was
    /// just removed always survives at least one full `ttl` window, so its
    /// clock keeps guarding against stale updates while they can still arrive.
    fn prune_disconnected_meta(&mut self, ttl: time::Duration) {
        let now = OffsetDateTime::now_utc();
        let states = &self.states;
        self.meta.retain(|client_id, meta| {
            states.contains_key(client_id) || now - meta.last_updated < ttl
        });
    }

    /// Returns a serializable update object which is representation of a current Awareness state.
    pub fn update(&self) -> Result<AwarenessUpdate, Error> {
        let clients = self.states.keys().cloned();
        self.update_with_clients(clients)
    }

    /// Returns a serializable update object which is representation of a current Awareness state.
    /// Unlike [Awareness::update], this method variant allows to prepare update only for a subset
    /// of known clients. These clients must all be known to a current [Awareness] instance,
    /// otherwise a [Error::ClientNotFound] error will be returned.
    pub fn update_with_clients<I: IntoIterator<Item = ClientID>>(
        &self,
        clients: I,
    ) -> Result<AwarenessUpdate, Error> {
        let mut res = HashMap::new();
        for client_id in clients {
            let clock = if let Some(meta) = self.meta.get(&client_id) {
                meta.clock
            } else {
                return Err(Error::ClientNotFound(client_id));
            };
            let json = if let Some(json) = self.states.get(&client_id) {
                json.clone()
            } else {
                String::from(NULL_STR)
            };
            res.insert(client_id, AwarenessUpdateEntry { clock, json });
        }
        Ok(AwarenessUpdate { clients: res })
    }

    /// Applies an update (incoming from remote channel or generated using [Awareness::update] /
    /// [Awareness::update_with_clients] methods) and modifies a state of a current instance.
    pub fn apply_update(&mut self, update: AwarenessUpdate) -> Result<(), Error> {
        self.apply_update_from(update, None)
    }

    /// Like [Awareness::apply_update], but tags the emitted [Event] with an `origin`
    /// identifying where the update came from (e.g. a connection ID), so that
    /// subscribers can avoid echoing an update back to its sender.
    pub fn apply_update_from(
        &mut self,
        update: AwarenessUpdate,
        origin: Option<u64>,
    ) -> Result<(), Error> {
        let mut added = Vec::new();
        let mut updated = Vec::new();
        let mut removed = Vec::new();

        for (client_id, entry) in update.clients {
            let mut clock = entry.clock;
            let is_null = entry.json.as_str() == NULL_STR;
            match self.meta.entry(client_id) {
                Entry::Occupied(mut e) => {
                    let prev = e.get();
                    let is_removed =
                        prev.clock == clock && is_null && self.states.contains_key(&client_id);
                    let is_new = prev.clock < clock;
                    if is_new || is_removed {
                        if is_null {
                            // never let a remote client remove this local state
                            if client_id == self.doc.client_id()
                                && self.states.contains_key(&client_id)
                            {
                                // remote client removed the local state. Do not remote state. Broadcast a message indicating
                                // that this client still exists by increasing the clock
                                clock += 1;
                            } else {
                                self.states.remove(&client_id);
                                if self.on_update.is_some() {
                                    removed.push(client_id);
                                }
                            }
                        } else {
                            match self.states.entry(client_id) {
                                Entry::Occupied(mut e) => {
                                    if self.on_update.is_some() {
                                        updated.push(client_id);
                                    }
                                    e.insert(entry.json);
                                }
                                Entry::Vacant(e) => {
                                    e.insert(entry.json);
                                    if self.on_update.is_some() {
                                        updated.push(client_id);
                                    }
                                }
                            }
                        }
                        e.insert(MetaClientState::new(clock));
                        true
                    } else {
                        false
                    }
                }
                Entry::Vacant(e) => {
                    e.insert(MetaClientState::new(clock));
                    self.states.insert(client_id, entry.json);
                    if self.on_update.is_some() {
                        added.push(client_id);
                    }
                    true
                }
            };
        }

        if let Some(eh) = self.on_update.as_ref() {
            if !added.is_empty() || !updated.is_empty() || !removed.is_empty() {
                let e = Event::with_origin(added, updated, removed, origin);
                eh.trigger(|cb| {
                    cb(self, &e);
                });
            }
        }

        Ok(())
    }
}

impl Default for Awareness {
    fn default() -> Self {
        Awareness::new(Doc::new())
    }
}

impl std::fmt::Debug for Awareness {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Awareness")
            .field("state", &self.states)
            .field("meta", &self.meta)
            .field("doc", &self.doc)
            .finish()
    }
}

/// A structure that represents an encodable state of an [Awareness] struct.
#[derive(Debug, Eq, PartialEq)]
pub struct AwarenessUpdate {
    pub(crate) clients: HashMap<ClientID, AwarenessUpdateEntry>,
}

impl Encode for AwarenessUpdate {
    fn encode<E: Encoder>(&self, encoder: &mut E) {
        encoder.write_var(self.clients.len());
        for (&client_id, e) in self.clients.iter() {
            encoder.write_var(client_id);
            encoder.write_var(e.clock);
            encoder.write_string(&e.json);
        }
    }
}

impl Decode for AwarenessUpdate {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, yrs::encoding::read::Error> {
        let len: usize = decoder.read_var()?;
        let mut clients = HashMap::with_capacity(len);
        for _ in 0..len {
            let client_id: ClientID = decoder.read_var()?;
            let clock: u32 = decoder.read_var()?;
            let json = decoder.read_string()?.to_string();
            clients.insert(client_id, AwarenessUpdateEntry { clock, json });
        }

        Ok(AwarenessUpdate { clients })
    }
}

/// A single client entry of an [AwarenessUpdate]. It consists of logical clock and JSON client
/// state represented as a string.
#[derive(Debug, Eq, PartialEq)]
pub struct AwarenessUpdateEntry {
    pub(crate) clock: u32,
    pub(crate) json: String,
}

/// Errors generated by an [Awareness] struct methods.
#[derive(Error, Debug)]
pub enum Error {
    /// Client ID was not found in [Awareness] metadata.
    #[error("client ID `{0}` not found")]
    ClientNotFound(ClientID),
}

#[derive(Debug, Clone)]
struct MetaClientState {
    clock: u32,
    /// When this entry was last created or bumped. Used by
    /// [Awareness::prune_disconnected_meta] to expire entries of
    /// disconnected clients. `OffsetDateTime` (not `std::time::Instant`)
    /// because y-sweet-worker compiles this crate to wasm32, where
    /// `Instant::now` panics.
    last_updated: OffsetDateTime,
}

impl MetaClientState {
    fn new(clock: u32) -> Self {
        MetaClientState {
            clock,
            last_updated: OffsetDateTime::now_utc(),
        }
    }
}

/// Event type emitted by an [Awareness] struct.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Event {
    added: Vec<ClientID>,
    updated: Vec<ClientID>,
    removed: Vec<ClientID>,
    /// Identifies where the change came from (e.g. a connection ID), if known.
    /// `None` for changes made directly on this instance (e.g. state removal
    /// on disconnect).
    origin: Option<u64>,
}

impl Event {
    pub fn new(added: Vec<ClientID>, updated: Vec<ClientID>, removed: Vec<ClientID>) -> Self {
        Self::with_origin(added, updated, removed, None)
    }

    pub fn with_origin(
        added: Vec<ClientID>,
        updated: Vec<ClientID>,
        removed: Vec<ClientID>,
        origin: Option<u64>,
    ) -> Self {
        Event {
            added,
            updated,
            removed,
            origin,
        }
    }

    /// The origin passed to [Awareness::apply_update_from], if any.
    pub fn origin(&self) -> Option<u64> {
        self.origin
    }

    /// Collection of new clients that have been added to an [Awareness] struct, that was not known
    /// before. Actual client state can be accessed via `awareness.clients().get(client_id)`.
    pub fn added(&self) -> &[ClientID] {
        &self.added
    }

    /// Collection of new clients that have been updated within an [Awareness] struct since the last
    /// update. Actual client state can be accessed via `awareness.clients().get(client_id)`.
    pub fn updated(&self) -> &[ClientID] {
        &self.updated
    }

    /// Collection of new clients that have been removed from [Awareness] struct since the last
    /// update.
    pub fn removed(&self) -> &[ClientID] {
        &self.removed
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};
    use yrs::Doc;

    fn update(
        recv: &mut Receiver<Event>,
        from: &Awareness,
        to: &mut Awareness,
    ) -> Result<Event, Box<dyn std::error::Error>> {
        let e = recv.try_recv()?;
        let u = from.update_with_clients([e.added(), e.updated(), e.removed()].concat())?;
        to.apply_update(u)?;
        Ok(e)
    }

    #[test]
    fn awareness() -> Result<(), Box<dyn std::error::Error>> {
        let (s1, mut o_local) = channel();
        let mut local = Awareness::new(Doc::with_client_id(1));
        let _sub_local = local.on_update(move |_, e| {
            s1.send(e.clone()).unwrap();
        });

        let (s2, o_remote) = channel();
        let mut remote = Awareness::new(Doc::with_client_id(2));
        let _sub_remote = local.on_update(move |_, e| {
            s2.send(e.clone()).unwrap();
        });

        local.set_local_state("{x:3}");
        let _e_local = update(&mut o_local, &local, &mut remote)?;
        assert_eq!(remote.clients()[&1], "{x:3}");
        assert_eq!(remote.meta[&1].clock, 1);
        assert_eq!(o_remote.try_recv()?.added, &[1]);

        local.set_local_state("{x:4}");
        let e_local = update(&mut o_local, &local, &mut remote)?;
        let e_remote = o_remote.try_recv()?;
        assert_eq!(remote.clients()[&1], "{x:4}");
        assert_eq!(e_remote, Event::new(vec![], vec![1], vec![]));
        assert_eq!(e_remote, e_local);

        local.clean_local_state();
        let e_local = update(&mut o_local, &local, &mut remote)?;
        let e_remote = o_remote.try_recv()?;
        assert_eq!(e_remote.removed.len(), 1);
        assert_eq!(local.clients().get(&1), None);
        assert_eq!(e_remote, e_local);
        Ok(())
    }

    #[test]
    fn apply_update_from_carries_origin() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = channel();
        let mut server = Awareness::new(Doc::with_client_id(0));
        let _sub = server.on_update(move |_, e| {
            tx.send(e.clone()).unwrap();
        });

        let mut update = HashMap::new();
        update.insert(
            1,
            AwarenessUpdateEntry {
                clock: 1,
                json: "{}".to_string(),
            },
        );
        server.apply_update_from(AwarenessUpdate { clients: update }, Some(42))?;
        assert_eq!(rx.try_recv()?.origin(), Some(42));

        let mut update = HashMap::new();
        update.insert(
            1,
            AwarenessUpdateEntry {
                clock: 2,
                json: "{\"x\":1}".to_string(),
            },
        );
        server.apply_update(AwarenessUpdate { clients: update })?;
        assert_eq!(rx.try_recv()?.origin(), None);
        Ok(())
    }

    /// Backdate a meta entry so it looks like the client disconnected `secs`
    /// seconds ago.
    fn backdate_meta(awareness: &mut Awareness, client_id: ClientID, secs: i64) {
        let meta = awareness.meta.get_mut(&client_id).unwrap();
        meta.last_updated -= time::Duration::seconds(secs);
    }

    #[test]
    fn remove_state_prunes_stale_disconnected_meta() -> Result<(), Box<dyn std::error::Error>> {
        let mut server = Awareness::new(Doc::with_client_id(0));

        // Three clients connect and announce a state.
        for client_id in 1..=3 {
            let mut update = HashMap::new();
            update.insert(
                client_id,
                AwarenessUpdateEntry {
                    clock: 1,
                    json: "{}".to_string(),
                },
            );
            server.apply_update(AwarenessUpdate { clients: update })?;
        }

        // Clients 1 and 2 disconnect; their meta tombstones age past the TTL.
        server.remove_state(1);
        server.remove_state(2);
        backdate_meta(&mut server, 1, 60);
        backdate_meta(&mut server, 2, 60);
        assert_eq!(server.meta.len(), 3);

        // The next disconnect prunes the stale tombstones; client 3's own
        // fresh tombstone and any connected clients' entries survive.
        server.remove_state(3);
        assert!(!server.meta.contains_key(&1));
        assert!(!server.meta.contains_key(&2));
        assert!(server.meta.contains_key(&3));
        Ok(())
    }

    #[test]
    fn stale_update_still_rejected_within_ttl() -> Result<(), Box<dyn std::error::Error>> {
        let mut server = Awareness::new(Doc::with_client_id(0));

        let mut update = HashMap::new();
        update.insert(
            1,
            AwarenessUpdateEntry {
                clock: 5,
                json: "{}".to_string(),
            },
        );
        server.apply_update(AwarenessUpdate { clients: update })?;

        // Client 1 disconnects. remove_state bumps its meta clock to 6 and
        // keeps the entry for the TTL window.
        server.remove_state(1);
        assert!(server.clients().get(&1).is_none());

        // A delayed update with an older clock must not resurrect the state.
        let mut stale = HashMap::new();
        stale.insert(
            1,
            AwarenessUpdateEntry {
                clock: 4,
                json: "{\"ghost\":true}".to_string(),
            },
        );
        server.apply_update(AwarenessUpdate { clients: stale })?;
        assert!(server.clients().get(&1).is_none());
        Ok(())
    }

    #[test]
    fn prune_keeps_connected_clients() -> Result<(), Box<dyn std::error::Error>> {
        let mut server = Awareness::new(Doc::with_client_id(0));

        let mut update = HashMap::new();
        update.insert(
            1,
            AwarenessUpdateEntry {
                clock: 1,
                json: "{}".to_string(),
            },
        );
        server.apply_update(AwarenessUpdate { clients: update })?;

        // Even with an ancient last_updated, a client that still has a state
        // is never pruned.
        backdate_meta(&mut server, 1, 3600);
        server.prune_disconnected_meta(DISCONNECTED_META_TTL);
        assert!(server.meta.contains_key(&1));
        assert_eq!(server.clients()[&1], "{}");
        Ok(())
    }
}
