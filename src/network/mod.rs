use ed25519_dalek::Keypair;
use std::{fmt::Debug, path::PathBuf, result, sync::Arc};
use thiserror::Error;
use tracing::{info, instrument};

mod database;
mod overlay;

use database::{ActorId, Database, Identity};
use overlay::Overlay;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    DatabaseError(#[from] database::Error),
}

#[derive(Debug)]
pub struct Network {
    database: Arc<Database>,
    overlay: Overlay,
}

impl Network {
    #[instrument(err)]
    pub async fn new<D>(database_path: D) -> Result<Self>
    where
        D: Debug + Into<PathBuf>,
    {
        let database = Arc::new(Database::new(database_path).await?);
        let overlay = Overlay::new(database.clone());

        Ok(Self { database, overlay })
    }

    #[instrument(err)]
    pub async fn generate_identity(&self) -> Result<ActorId> {
        // TODO: review the source of randomness

        info!("Generating a new identity...");
        let identity = Identity {
            keypair: Keypair::generate(&mut rand::thread_rng()),
        };
        let id = identity.id();
        self.database.insert_identity(&id, &identity).await?;
        info!("New identity: {}", id);

        Ok(id)
    }

    #[instrument(err)]
    pub async fn start(&self, identity_id: &ActorId) -> Result<()> {
        info!("Starting network with identity: {}", identity_id);
        self.overlay
            .start(self.database.get_identity(identity_id).await?.unwrap());

        Ok(())
    }
}
