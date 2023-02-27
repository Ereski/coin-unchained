// TODO: make the database futures-aware

use chrono::NaiveDateTime;
use ed25519_dalek::{Keypair, PublicKey, Signature};
use lmdb_rs::core::{
    DbCreate, DbHandle, EnvCreateNoLock, EnvCreateWriteMap, Environment,
    MdbError, MdbResult,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_derive::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
    fmt::{self, Debug, Display, Formatter},
    net::SocketAddr,
    path::PathBuf,
    result,
};
use thiserror::Error;
use tokio::{sync::RwLock, task};
use tracing::{info, instrument};

const IDENTITIES_DATABASE_NAME: &str = "identities";
const ACTORS_DATABASE_NAME: &str = "actors";
const TRANSACTIONS_DATABASE_NAME: &str = "transactions";

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to initialize {0}: {1}")]
    DatabaseInitializationError(
        &'static str,
        #[source] lmdb_rs::core::MdbError,
    ),

    #[error("failed to fetch status for {0} database: {1}")]
    StatusFetchFailed(&'static str, #[source] lmdb_rs::core::MdbError),

    #[error("failed to insert identity into the database: {0}")]
    IdentityInsertionError(#[source] lmdb_rs::core::MdbError),

    #[error("failed to get {0} with key {1} from the database: {2}")]
    FetchError(&'static str, String, #[source] lmdb_rs::core::MdbError),

    #[error(r#"failed to deserialize {0}: "{1}""#)]
    DeserializationError(&'static str, String),
}

#[derive(Deserialize, Serialize)]
pub struct Identity {
    pub keypair: Keypair,
}

impl Identity {
    pub fn id(&self) -> ActorId {
        let mut hash = [0_u8; 32];
        // Have to do this copy cause `GenericArray` does not expose its
        // underlying array :/
        hash.copy_from_slice(&Sha3_256::digest(
            &self.keypair.public.to_bytes(),
        ));

        ActorId(hash)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ActorId([u8; 32]);

impl Display for ActorId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

#[derive(Deserialize, Serialize)]
#[non_exhaustive]
pub enum Actor {
    Allowed {
        public_key: Option<PublicKey>,
        last_known_address: SocketAddr,
        last_seen: NaiveDateTime,
    },
}

#[derive(Deserialize, Serialize)]
pub struct TransactionId {
    pub actor: ActorId,
    pub index: u64,
}

#[derive(Deserialize, Serialize)]
pub struct Transaction {
    pub application: (),
    pub message: Vec<u8>,
    pub nonce: u64,
    pub signature: Signature,
}

pub struct Database {
    path: PathBuf,

    environment: RwLock<Environment>,

    identities: DbHandle,
    actors: DbHandle,
    transactions: DbHandle,
}

impl Debug for Database {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, r#""{}""#, self.path.display())
    }
}

impl Database {
    pub async fn new<P>(path: P) -> Result<Self>
    where
        P: Debug + Into<PathBuf>,
    {
        let path = path.into();
        task::spawn_blocking(move || Self::sync_new(path))
            .await
            .unwrap()
    }

    #[instrument(err, name = "new")]
    fn sync_new(path: PathBuf) -> Result<Self> {
        info!("Opening environment");
        let environment = Environment::new()
            .flags(EnvCreateNoLock | EnvCreateWriteMap)
            .max_dbs(3)
            .autocreate_dir(true)
            .open(&path, 0o600)
            .map_err(|x| {
                Error::DatabaseInitializationError("environment", x)
            })?;

        info!("Opening subdatabases");
        let identities = environment
            .create_db(IDENTITIES_DATABASE_NAME, DbCreate)
            .map_err(|x| {
                Error::DatabaseInitializationError("identity database", x)
            })?;
        let actors = environment
            .create_db(ACTORS_DATABASE_NAME, DbCreate)
            .map_err(|x| {
                Error::DatabaseInitializationError("actor database", x)
            })?;
        let transactions = environment
            .create_db(TRANSACTIONS_DATABASE_NAME, DbCreate)
            .map_err(|x| {
                Error::DatabaseInitializationError("transaction database", x)
            })?;

        info!(
            "Found {} identities",
            environment
                .get_reader()
                .and_then(|x| x.bind(&identities).stat())
                .map_err(|x| Error::StatusFetchFailed("identity", x))?
                .ms_entries
        );
        info!(
            "Found {} actors",
            environment
                .get_reader()
                .and_then(|x| x.bind(&actors).stat())
                .map_err(|x| Error::StatusFetchFailed("actor", x))?
                .ms_entries
        );
        info!(
            "Found {} transactions",
            environment
                .get_reader()
                .and_then(|x| x.bind(&transactions).stat())
                .map_err(|x| Error::StatusFetchFailed("transaction", x))?
                .ms_entries
        );

        info!("Database ready");

        Ok(Self {
            path,

            environment: RwLock::new(environment),
            identities,
            actors,
            transactions,
        })
    }

    pub async fn insert_identity(
        &self,
        id: &ActorId,
        identity: &Identity,
    ) -> Result<()> {
        self.environment
            .write()
            .await
            .new_transaction()
            .and_then(|x| {
                x.bind(&self.identities)
                    .insert(&(&id.0 as &[u8]), &Self::serialize(identity))?;
                x.commit()?;

                Ok(())
            })
            .map_err(Error::IdentityInsertionError)
    }

    pub async fn get_identity(&self, id: &ActorId) -> Result<Option<Identity>> {
        let identity_raw = self
            .environment
            .read()
            .await
            .get_reader()
            .and_then(|x| {
                Self::optionalize_lmdb_get(
                    x.bind(&self.identities).get(&(&id.0 as &[u8])),
                )
            })
            .map_err(|x| Error::FetchError("identity", id.to_string(), x))?;

        if let Some(identity_raw) = identity_raw {
            Ok(Some(Self::deserialize(identity_raw).map_err(|_| {
                Error::DeserializationError("identity", id.to_string())
            })?))
        } else {
            Ok(None)
        }
    }

    fn deserialize<T>(data: &[u8]) -> bincode::Result<T>
    where
        T: DeserializeOwned,
    {
        bincode::deserialize(&data)
    }

    fn serialize<T>(data: T) -> Vec<u8>
    where
        T: Serialize,
    {
        bincode::serialize(&data).unwrap()
    }

    fn optionalize_lmdb_get<T>(res: MdbResult<T>) -> MdbResult<Option<T>> {
        match res {
            Ok(x) => Ok(Some(x)),
            Err(MdbError::NotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
