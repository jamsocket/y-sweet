use crate::api_types::Authorization;
use crate::sync::{
    self, awareness::Awareness, DefaultProtocol, Message, Protocol, SyncMessage, MSG_SYNC,
    MSG_SYNC_UPDATE,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use yrs::{
    block::ClientID,
    encoding::write::Write,
    updates::{
        decoder::Decode,
        encoder::{Encode, Encoder, EncoderV1},
    },
    ReadTxn, Subscription, Transact, Update,
};

// TODO: this is an implementation detail and should not be exposed.
pub const DOC_NAME: &str = "doc";

#[cfg(not(feature = "sync"))]
type Callback = Arc<dyn Fn(&[u8]) + 'static>;

#[cfg(feature = "sync")]
type Callback = Arc<dyn Fn(&[u8]) + 'static + Send + Sync>;

const SYNC_STATUS_MESSAGE: u8 = 102;

/// Source of unique per-process connection IDs, used as the origin of awareness
/// updates so a connection can recognize (and skip) its own updates when they
/// are broadcast.
static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

pub struct DocConnection {
    awareness: Arc<RwLock<Awareness>>,
    #[allow(unused)] // acts as RAII guard
    doc_subscription: Subscription,
    #[allow(unused)] // acts as RAII guard
    awareness_subscription: Subscription,
    authorization: Authorization,
    callback: Callback,
    closed: Arc<OnceLock<()>>,
    connection_id: u64,

    /// If the client sends an awareness state, this will be set to its client ID.
    /// It is used to clear the awareness state when a client disconnects.
    client_id: OnceLock<ClientID>,
}

impl DocConnection {
    #[cfg(not(feature = "sync"))]
    pub fn new<F>(
        awareness: Arc<RwLock<Awareness>>,
        authorization: Authorization,
        callback: F,
    ) -> Self
    where
        F: Fn(&[u8]) + 'static,
    {
        Self::new_inner(awareness, authorization, Arc::new(callback))
    }

    #[cfg(feature = "sync")]
    pub fn new<F>(
        awareness: Arc<RwLock<Awareness>>,
        authorization: Authorization,
        callback: F,
    ) -> Self
    where
        F: Fn(&[u8]) + 'static + Send + Sync,
    {
        Self::new_inner(awareness, authorization, Arc::new(callback))
    }

    pub fn new_inner(
        awareness: Arc<RwLock<Awareness>>,
        authorization: Authorization,
        callback: Callback,
    ) -> Self {
        let closed = Arc::new(OnceLock::new());
        let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);

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
                let closed = closed.clone();
                doc.observe_update_v1(move |_, event| {
                    if closed.get().is_some() {
                        return;
                    }
                    // https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/net/broadcast.rs#L47-L52
                    let mut encoder = EncoderV1::new();
                    encoder.write_var(MSG_SYNC);
                    encoder.write_var(MSG_SYNC_UPDATE);
                    encoder.write_buf(&event.update);
                    let msg = encoder.to_vec();
                    callback(&msg);
                })
                .unwrap()
            };

            let callback = callback.clone();
            let closed = closed.clone();
            let awareness_subscription = awareness.on_update(move |awareness, e| {
                if closed.get().is_some() {
                    return;
                }

                // Don't echo an awareness update back to the connection that
                // sent it; the sender already has that state.
                if e.origin() == Some(connection_id) {
                    return;
                }

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
            authorization,
            callback,
            connection_id,
            client_id: OnceLock::new(),
            closed,
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
        let can_write = matches!(self.authorization, Authorization::Full);
        let a = &self.awareness;
        match msg {
            Message::Sync(msg) => match msg {
                SyncMessage::SyncStep1(sv) => {
                    let awareness = a.read().unwrap();
                    protocol.handle_sync_step1(&awareness, sv)
                }
                SyncMessage::SyncStep2(update) => {
                    if can_write {
                        let mut awareness = a.write().unwrap();
                        protocol.handle_sync_step2(&mut awareness, Update::decode_v1(&update)?)
                    } else {
                        Err(sync::Error::PermissionDenied {
                            reason: "Token does not have write access".to_string(),
                        })
                    }
                }
                SyncMessage::Update(update) => {
                    if can_write {
                        let mut awareness = a.write().unwrap();
                        protocol.handle_update(&mut awareness, Update::decode_v1(&update)?)
                    } else {
                        Err(sync::Error::PermissionDenied {
                            reason: "Token does not have write access".to_string(),
                        })
                    }
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
                protocol.handle_awareness_update_from(&mut awareness, update, Some(self.connection_id))
            }
            Message::Custom(SYNC_STATUS_MESSAGE, data) => {
                // Respond to the client with the same payload it sent.
                Ok(Some(Message::Custom(SYNC_STATUS_MESSAGE, data)))
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
        self.closed.set(()).unwrap();

        // If this client had an awareness state, remove it.
        if let Some(client_id) = self.client_id.get() {
            let mut awareness = self.awareness.write().unwrap();
            awareness.remove_state(*client_id);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::awareness::{AwarenessUpdate, AwarenessUpdateEntry};
    use crate::sync::MSG_AWARENESS;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use yrs::Doc;

    type Frames = Arc<Mutex<Vec<Vec<u8>>>>;

    fn new_conn(awareness: &Arc<RwLock<Awareness>>) -> (DocConnection, Frames) {
        let frames: Frames = Arc::default();
        let conn = {
            let frames = frames.clone();
            DocConnection::new(awareness.clone(), Authorization::Full, move |bytes: &[u8]| {
                frames.lock().unwrap().push(bytes.to_vec());
            })
        };
        // Discard the initial handshake (sync step 1 + initial awareness).
        frames.lock().unwrap().clear();
        (conn, frames)
    }

    fn awareness_message(client_id: ClientID, clock: u32, json: &str) -> Message {
        let mut clients = HashMap::new();
        clients.insert(
            client_id,
            AwarenessUpdateEntry {
                clock,
                json: json.to_string(),
            },
        );
        Message::Awareness(AwarenessUpdate { clients })
    }

    #[test]
    fn awareness_update_is_not_echoed_to_sender() {
        let awareness = Arc::new(RwLock::new(Awareness::new(Doc::new())));
        let (conn_a, frames_a) = new_conn(&awareness);
        let (_conn_b, frames_b) = new_conn(&awareness);

        conn_a
            .handle_msg(&DefaultProtocol, awareness_message(7, 1, "{}"))
            .unwrap();

        assert!(
            frames_a.lock().unwrap().is_empty(),
            "sender must not receive its own awareness update"
        );
        let frames_b = frames_b.lock().unwrap();
        assert_eq!(frames_b.len(), 1, "other connections must receive the update");
        assert_eq!(frames_b[0][0], MSG_AWARENESS);
    }

    #[test]
    fn awareness_removal_on_drop_reaches_remaining_connections() {
        let awareness = Arc::new(RwLock::new(Awareness::new(Doc::new())));
        let (_conn_a, frames_a) = new_conn(&awareness);
        let (conn_b, frames_b) = new_conn(&awareness);

        conn_b
            .handle_msg(&DefaultProtocol, awareness_message(7, 1, "{}"))
            .unwrap();
        frames_a.lock().unwrap().clear();

        // Dropping B removes client 7's state; that change has no connection
        // origin, so remaining connections must be notified.
        drop(conn_b);
        drop(frames_b);

        let frames_a = frames_a.lock().unwrap();
        assert_eq!(frames_a.len(), 1);
        assert_eq!(frames_a[0][0], MSG_AWARENESS);
    }
}
