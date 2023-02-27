use crate::network::database::{
    ActorId, Database, Identity, Transaction, TransactionId,
};
use ed25519_dalek::{PublicKey, Signature};
use futures::stream::{FuturesUnordered, StreamExt};
use serde_derive::{Deserialize, Serialize};
use std::{
    fmt::{self, Debug, Formatter},
    mem,
    net::SocketAddr,
    sync::{mpsc, Arc},
};
use tokio::sync::Mutex;

pub struct Overlay {
    database: Arc<Database>,

    // This needs to be wrapped in an `Arc` cause of some sync traits we
    // implement
    connections: Arc<Mutex<Vec<Connection>>>,
}

impl Debug for Overlay {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (sender, receiver) = mpsc::channel();
        let connections = self.connections.clone();
        tokio::spawn(async move {
            let len = connections.lock().await.len();
            let _ = sender.send(len);
        });

        write!(f, "{} peers", receiver.recv().unwrap())
    }
}

impl Overlay {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,

            connections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(&self, identity: Identity) {
        unimplemented!()
    }

    pub async fn stop(&self) {
        Self::do_stop(self.connections.lock().await.drain(..)).await;
    }

    async fn do_stop<I>(connections: I)
    where
        I: IntoIterator<Item = Connection>,
    {
        connections
            .into_iter()
            .map(|x| x.close())
            .collect::<FuturesUnordered<_>>()
            .for_each(|_| async {})
            .await;
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        let connections = self.connections.clone();
        tokio::spawn(async move {
            Self::do_stop(connections.lock().await.drain(..)).await
        });
    }
}

#[derive(Debug)]
struct Connection {}

impl Connection {
    async fn close(self) {
        unimplemented!()
    }
}

#[derive(Deserialize, Serialize)]
enum Message {
    Identify {
        public_key: PublicKey,
    },
    Challenge {
        id: ActorId,
        payload: [u8; 8],
    },
    ChallengeResponse {
        id: ActorId,
        payload: [u8; 8],
        signature: Signature,
    },
    Allow {
        id: ActorId,
        tip: u64,
    },
    Deny {
        id: ActorId,
    },
    PushTransaction {
        id: TransactionId,
        body: Transaction,
    },
    UpdateActorAddress {
        id: ActorId,
        address: SocketAddr,
    },
}
