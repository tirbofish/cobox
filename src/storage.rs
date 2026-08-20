use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use esp_idf_svc::sys::EspError;

use crate::blob::BlobConfig;

const NAMESPACE: &str = "cobox";
const BLOB_KEY: &str = "blob";

pub struct BlobStore {
    nvs: EspDefaultNvs,
}

impl BlobStore {
    pub fn new(partition: EspDefaultNvsPartition) -> Result<Self, EspError> {
        let nvs = EspDefaultNvs::new(partition, NAMESPACE, true)?;
        Ok(Self { nvs })
    }

    pub fn load(&self) -> Result<Option<BlobConfig>, EspError> {
        let Some(length) = self.nvs.blob_len(BLOB_KEY)? else {
            return Ok(None);
        };
        if length != BlobConfig::SERIALIZED_LEN {
            log::warn!("discarding saved blob profile with unexpected length: {length}");
            return Ok(None);
        }

        let mut bytes = [0; BlobConfig::SERIALIZED_LEN];
        let Some(bytes) = self.nvs.get_blob(BLOB_KEY, &mut bytes)? else {
            return Ok(None);
        };

        match BlobConfig::deserialize(bytes) {
            Ok(config) => Ok(Some(config)),
            Err(error) => {
                log::warn!("discarding invalid saved blob profile: {error:?}");
                Ok(None)
            }
        }
    }

    pub fn save(&self, config: BlobConfig) -> Result<(), EspError> {
        let bytes = config.serialize();
        debug_assert!(matches!(
            BlobConfig::deserialize(&bytes),
            Ok(restored) if restored == config
        ));
        self.nvs.set_blob(BLOB_KEY, &bytes)
    }
}
