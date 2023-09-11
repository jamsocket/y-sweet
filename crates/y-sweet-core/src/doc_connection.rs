use crate::sync::{
    self,
    awareness::{Awareness, Event},
    DefaultProtocol, Message, Protocol, SyncMessage,
};
use std::sync::{Arc, OnceLock, RwLock};
use yrs::{
    block::ClientID,
    updates::{decoder::Decode, encoder::Encode},
    ReadTxn, Subscription, Transact, Update, UpdateSubscription,
};

// TODO: this is an implementation detail and should not be exposed.
pub const DOC_NAME: &str = "doc";

type AwarenessSubscription = Subscription<Arc<dyn Fn(&Awareness, &Event)>>;

#[cfg(target_arch = "wasm32")]
type Callback = Arc<dyn Fn(&[u8]) + 'static>;

#[cfg(not(target_arch = "wasm32"))]
type Callback = Arc<dyn Fn(&[u8]) + 'static + Send + Sync>;

pub struct DocConnection {
    awareness: Arc<RwLock<Awareness>>,
    #[allow(unused)] // acts as RAII guard
    doc_subscription: UpdateSubscription,
    #[allow(unused)] // acts as RAII guard
    awareness_subscription: AwarenessSubscription,
    callback: Callback,

    /// If the client sends an awareness state, this will be set to its client ID.
    /// It is used to clear the awareness state when a client disconnects.
    client_id: OnceLock<ClientID>,
}

impl DocConnection {
    #[cfg(target_arch = "wasm32")]
    pub fn new<F>(awareness: Arc<RwLock<Awareness>>, callback: F) -> Self
    where
        F: Fn(&[u8]) + 'static,
    {
        Self::new_inner(awareness, Arc::new(callback))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new<F>(awareness: Arc<RwLock<Awareness>>, callback: F) -> Self
    where
        F: Fn(&[u8]) + 'static + Send + Sync,
    {
        Self::new_inner(awareness, Arc::new(callback))
    }

    pub fn new_inner(awareness: Arc<RwLock<Awareness>>, callback: Callback) -> Self {
        let (doc_subscription, awareness_subscription) = {
            let mut awareness = awareness.write().unwrap();

            // Initial handshake is based on this:
            // https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/sync.rs#L45-L54

            {
                // Send a server-side state vector, so that the client can send
                // updates that happened offline.
                let sv = awareness.doc().transact().state_vector();
                let sync_step_1 = Message::Sync(SyncMessage::SyncStep1(sv)).encode_v1();
                callback(&sync_step_1);
            }

            {
                // Send the initial awareness state.
                let update = awareness.update().unwrap();
                let awareness = Message::Awareness(update).encode_v1();
                callback(&awareness);
            }

            let doc_subscription = {
                let doc = awareness.doc();
                let callback = callback.clone();
                doc.observe_update_v1(move |_, event| {
                    // TODO: avoid allocation: https://github.com/y-crdt/y-sync/blob/master/src/net/broadcast.rs#L48
                    let msg = Message::Sync(SyncMessage::Update(event.update.clone()));
                    let msg = msg.encode_v1();
                    callback(&msg);
                })
                .unwrap()
            };

            let callback = callback.clone();
            let awareness_subscription = awareness.on_update(move |awareness, e| {
                // https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/net/broadcast.rs#L59
                let added = e.added();
                let updated = e.updated();
                let removed = e.removed();
                let mut changed = Vec::with_capacity(added.len() + updated.len() + removed.len());
                changed.extend_from_slice(added);
                changed.extend_from_slice(updated);
                changed.extend_from_slice(removed);

                if let Ok(u) = awareness.update_with_clients(changed) {
                    let msg = Message::Awareness(u).encode_v1();
                    callback(&msg);
                }
            });

            (doc_subscription, awareness_subscription)
        };

        Self {
            awareness,
            doc_subscription,
            awareness_subscription,
            callback,
            client_id: OnceLock::new(),
        }
    }

    pub async fn send(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        let msg = Message::decode_v1(update)?;
        let result = self.handle_msg(&DefaultProtocol, msg)?;

        if let Some(result) = result {
            let msg = result.encode_v1();
            (self.callback)(&msg);
        }

        Ok(())
    }

    // Adapted from:
    // https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/net/conn.rs#L184C1-L222C1
    pub fn handle_msg<P: Protocol>(
        &self,
        protocol: &P,
        msg: Message,
    ) -> Result<Option<Message>, sync::Error> {
        let a = &self.awareness;
        match msg {
            Message::Sync(msg) => match msg {
                SyncMessage::SyncStep1(sv) => {
                    let awareness = a.read().unwrap();
                    protocol.handle_sync_step1(&awareness, sv)
                }
                SyncMessage::SyncStep2(update) => {
                    let mut awareness = a.write().unwrap();
                    protocol.handle_sync_step2(&mut awareness, Update::decode_v1(&update)?)
                }
                SyncMessage::Update(update) => {
                    let mut awareness = a.write().unwrap();
                    protocol.handle_update(&mut awareness, Update::decode_v1(&update)?)
                }
            },
            Message::Auth(reason) => {
                let awareness = a.read().unwrap();
                protocol.handle_auth(&awareness, reason)
            }
            Message::AwarenessQuery => {
                let awareness = a.read().unwrap();
                protocol.handle_awareness_query(&awareness)
            }
            Message::Awareness(update) => {
                if update.clients.len() == 1 {
                    let client_id = update.clients.keys().next().unwrap();
                    self.client_id.get_or_init(|| *client_id);
                } else {
                    tracing::warn!("Received awareness update with more than one client");
                }
                let mut awareness = a.write().unwrap();
                protocol.handle_awareness_update(&mut awareness, update)
            }
            Message::Custom(tag, data) => {
                let mut awareness = a.write().unwrap();
                protocol.missing_handle(&mut awareness, tag, data)
            }
        }
    }
}

impl Drop for DocConnection {
    fn drop(&mut self) {
        // If this client had an awareness state, remove it.
        if let Some(client_id) = self.client_id.get() {
            let mut awareness = self.awareness.write().unwrap();
            awareness.remove_state(*client_id);
        }
    }
}
