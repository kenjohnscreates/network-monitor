//! Thin wrapper over `maxminddb` for country lookups.
//!
//! Designed to fail soft: if no DB is loaded, every lookup returns `None`.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use maxminddb::Reader;

#[derive(Debug, thiserror::Error)]
pub enum GeoipError {
    #[error("maxminddb: {0}")]
    Db(#[from] maxminddb::MaxMindDbError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(serde::Deserialize)]
struct CountryRecord {
    country: Option<CountryNode>,
}

#[derive(serde::Deserialize)]
struct CountryNode {
    iso_code: Option<String>,
}

#[derive(Clone)]
pub struct GeoipDb {
    reader: Arc<Reader<Vec<u8>>>,
}

impl GeoipDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GeoipError> {
        let reader = Reader::open_readfile(path.as_ref())?;
        Ok(Self {
            reader: Arc::new(reader),
        })
    }

    pub fn lookup_country(&self, ip: IpAddr) -> Option<String> {
        let record: Option<CountryRecord> = self.reader.lookup(ip).ok().flatten();
        record?.country?.iso_code
    }
}
