use std::sync::{Arc, RwLock};
use y_sync::{
    awareness::{Awareness, Event},
    sync::{self, DefaultProtocol, Message, Protocol, SyncMessage},
};
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Subscription, TransactionMut, Update, UpdateEvent,
};

// TODO: this is an implementation detail and should not be exposed.
pub const DOC_NAME: &str = "doc";

type UpdateSubscription = Subscription<Arc<dyn Fn(&TransactionMut<'_>, &UpdateEvent)>>;
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

    pub fn new_inner(awareness: Arc<RwLock<Awareness>>, callback: Callback) -> Self
    {
        let (doc_subscription, awareness_subscription) = {
            let mut awareness = awareness.write().unwrap();

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
        }
    }

    pub async fn send(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        let msg = Message::decode_v1(update)?;
        let result = handle_msg(&DefaultProtocol, &self.awareness, msg)?;

        if let Some(result) = result {
            let msg = result.encode_v1();
            (self.callback)(&msg);
        }

        Ok(())
    }
}

// Adapted from:
// https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/net/conn.rs#L184C1-L222C1
fn handle_msg<P: Protocol>(
    protocol: &P,
    a: &Arc<RwLock<Awareness>>,
    msg: Message,
) -> Result<Option<Message>, sync::Error> {
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
            #[cfg(target_arch = "wasm32")]
            {
                Ok(None)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let awareness = a.read().unwrap();
                protocol.handle_awareness_query(&awareness)
            }
        }
        Message::Awareness(update) => {
            #[cfg(target_arch = "wasm32")]
            {
                Ok(None)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut awareness = a.write().unwrap();
                protocol.handle_awareness_update(&mut awareness, update)
            }
        }
        Message::Custom(tag, data) => {
            let mut awareness = a.write().unwrap();
            protocol.missing_handle(&mut awareness, tag, data)
        }
    }
}
