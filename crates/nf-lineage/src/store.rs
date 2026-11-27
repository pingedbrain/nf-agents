//! Lineage stores: the [`LineageStore`] contract and its `object_store` implementation.

use std::sync::Arc;

use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt, stream};
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreExt};
use serde::Serialize;

use crate::{FindQuery, Lid, LineageError, Record};

const RECORD_FILE: &str = ".data.json";
const HISTORY_DIR: &str = ".history";

/// Read-only access to a lineage store. Dyn-compatible: other backends implement it and every
/// function of this crate takes `&dyn LineageStore`.
#[async_trait]
pub trait LineageStore: Send + Sync {
    /// The runs listed in `.history`, sorted by timestamp text then LID (`[]` without history).
    async fn list_runs(&self) -> Result<Vec<RunSummary>, LineageError>;
    /// The record behind `lid` (a fragment other than `output` is ignored).
    async fn get(&self, lid: &Lid) -> Result<Record, LineageError>;
    /// The LIDs of every record matching `query`, sorted and de-duplicated.
    async fn find(&self, query: &FindQuery) -> Result<Vec<Lid>, LineageError>;
}

/// One `.history` entry (`nextflow lineage list` row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunSummary {
    /// When the run started, as written by Nextflow (`yyyy-MM-dd HH:mm:ss z`).
    pub timestamp: String,
    /// The run name, e.g. `friendly_ride`.
    pub run_name: String,
    /// The session UUID.
    pub session_id: String,
    /// The run's WorkflowRun LID.
    pub lid: Lid,
}

/// A lineage store (Nextflow's `.lineage` directory) on any `object_store` backend.
#[derive(Debug, Clone)]
pub struct ObjectLineageStore {
    store: Arc<dyn ObjectStore>,
    prefix: Path,
}

impl ObjectLineageStore {
    /// Wraps an object store; `prefix` is the lineage directory inside it.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use nf_lineage::object_store::{memory::InMemory, path::Path};
    /// use nf_lineage::{FindQuery, LineageStore, ObjectLineageStore};
    ///
    /// let store = ObjectLineageStore::new(Arc::new(InMemory::new()), Path::parse("lineage")?);
    /// let query = FindQuery::parse_args(["type=WorkflowRun"])?;
    /// let runs = futures::executor::block_on(store.find(&query))?;
    /// assert!(runs.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(store: Arc<dyn ObjectStore>, prefix: Path) -> Self {
        Self { store, prefix }
    }

    /// Opens a local directory, `file://…`, `s3://bucket[/prefix]`, `gs://…` or `az://…`
    /// (`abfs[s]://…`) location. Cloud settings come from the environment (`AWS_*`, `GOOGLE_*`,
    /// `AZURE_*`).
    ///
    /// # Errors
    /// [`LineageError::NotFound`] for a missing local directory, [`LineageError::Unsupported`]
    /// for another scheme (or a cloud scheme whose feature is off), [`LineageError::Storage`]
    /// for an invalid cloud configuration.
    pub fn open(location: &str) -> Result<Self, LineageError> {
        Self::open_with_options(location, std::iter::empty::<(&str, String)>())
    }

    /// Like [`ObjectLineageStore::open`], with explicit `object_store` config keys (e.g.
    /// `aws_endpoint`, `aws_allow_http`) layered over the environment.
    ///
    /// # Errors
    /// As [`ObjectLineageStore::open`]; an unknown key (or any key for a local store) is
    /// [`LineageError::Unsupported`].
    pub fn open_with_options<K: AsRef<str>, V: Into<String>>(
        location: &str,
        options: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, LineageError> {
        let options: Vec<(String, String)> = options
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_ascii_lowercase(), v.into()))
            .collect();
        let url = match url::Url::parse(location) {
            // A one-letter scheme is a Windows drive (`C:\…`).
            Ok(url) if url.scheme().len() > 1 => url,
            _ => return Self::local(std::path::Path::new(location), &options),
        };
        if url.scheme() == "file" {
            let dir = url.to_file_path().map_err(|()| {
                LineageError::Unsupported(format!("file URL without a local path: {location}"))
            })?;
            return Self::local(&dir, &options);
        }
        let store: Result<Arc<dyn ObjectStore>, LineageError> = match url.scheme() {
            #[cfg(feature = "aws")]
            "s3" | "s3a" => {
                use object_store::aws::AmazonS3Builder as Builder;
                let builder = configure(
                    Builder::from_env().with_url(location),
                    &options,
                    |b, k, v| b.with_config(k, v),
                )?;
                Ok(Arc::new(builder.build()?))
            }
            #[cfg(feature = "gcp")]
            "gs" => {
                use object_store::gcp::GoogleCloudStorageBuilder as Builder;
                let builder = configure(
                    Builder::from_env().with_url(location),
                    &options,
                    |b, k, v| b.with_config(k, v),
                )?;
                Ok(Arc::new(builder.build()?))
            }
            #[cfg(feature = "azure")]
            "az" | "adl" | "azure" | "abfs" | "abfss" => {
                use object_store::azure::MicrosoftAzureBuilder as Builder;
                let builder = configure(
                    Builder::from_env().with_url(location),
                    &options,
                    |b, k, v| b.with_config(k, v),
                )?;
                Ok(Arc::new(builder.build()?))
            }
            other => Err(LineageError::Unsupported(format!(
                "store scheme '{other}' (or its cargo feature `aws`/`gcp`/`azure` is off)"
            ))),
        };
        let prefix = Path::from_url_path(url.path()).map_err(object_store::Error::from)?;
        Ok(Self::new(store?, prefix))
    }

    fn local(dir: &std::path::Path, options: &[(String, String)]) -> Result<Self, LineageError> {
        if let Some((key, _)) = options.first() {
            return Err(LineageError::Unsupported(format!(
                "store option '{key}' for a local store"
            )));
        }
        if !dir.is_dir() {
            return Err(LineageError::NotFound {
                target: dir.display().to_string(),
            });
        }
        let store = LocalFileSystem::new_with_prefix(dir)?;
        Ok(Self::new(Arc::new(store), Path::default()))
    }

    /// `<prefix>/<relative>`; `Path::parse` keeps `#` verbatim (`Path::from` would encode it).
    fn key(&self, relative: &str) -> Result<Path, LineageError> {
        let full = if self.prefix.as_ref().is_empty() {
            relative.to_owned()
        } else {
            format!("{}/{relative}", self.prefix)
        };
        Ok(Path::parse(full).map_err(object_store::Error::from)?)
    }

    async fn read(&self, location: &Path) -> Result<Vec<u8>, LineageError> {
        Ok(self.store.get(location).await?.bytes().await?.to_vec())
    }

    async fn list(&self, prefix: &Path) -> Result<Vec<ObjectMeta>, LineageError> {
        Ok(self.store.list(Some(prefix)).try_collect().await?)
    }

    async fn matching(
        &self,
        lid: Result<Lid, LineageError>,
        location: Path,
        query: &FindQuery,
    ) -> Result<Option<Lid>, LineageError> {
        let lid = lid?;
        let record = Record::parse(&self.read(&location).await?, location.as_ref())?;
        Ok(query.matches(&record).then_some(lid))
    }

    /// The LID of a record object (`<prefix>/<key>/.data.json` outside `.history`).
    fn record_lid(&self, location: &Path) -> Option<Result<Lid, LineageError>> {
        let mut parts: Vec<String> = location
            .prefix_match(&self.prefix)?
            .map(|p| p.as_ref().to_owned())
            .collect();
        if parts.pop()? != RECORD_FILE || parts.first()? == HISTORY_DIR {
            return None;
        }
        Some(
            Lid::parse(&format!("lid://{}", parts.join("/"))).map_err(|e| {
                LineageError::InvalidFormat {
                    path: location.to_string(),
                    message: e.to_string(),
                }
            }),
        )
    }
}

#[cfg(any(feature = "aws", feature = "gcp", feature = "azure"))]
fn configure<B, K: std::str::FromStr>(
    builder: B,
    options: &[(String, String)],
    set: impl Fn(B, K, String) -> B,
) -> Result<B, LineageError> {
    options
        .iter()
        .try_fold(builder, |b, (key, value)| match key.parse() {
            Ok(k) => Ok(set(b, k, value.clone())),
            Err(_) => Err(LineageError::Unsupported(format!(
                "unknown store option '{key}'"
            ))),
        })
}

/// One `.history` file: `timestamp \t runName \t sessionId \t lid` (Groovy `tokenize`: empty
/// tokens are dropped).
fn parse_history(bytes: &[u8], origin: &str) -> Result<RunSummary, LineageError> {
    let text = String::from_utf8_lossy(bytes);
    let line = text.trim_end();
    let invalid = || LineageError::InvalidFormat {
        path: origin.to_owned(),
        message: format!("not a valid history entry: `{line}`"),
    };
    let tokens: Vec<&str> = line.split('\t').filter(|t| !t.is_empty()).collect();
    let [timestamp, run_name, session_id, lid] = tokens.as_slice() else {
        return Err(invalid());
    };
    Ok(RunSummary {
        timestamp: (*timestamp).to_owned(),
        run_name: (*run_name).to_owned(),
        session_id: (*session_id).to_owned(),
        lid: Lid::parse(lid).map_err(|_| invalid())?,
    })
}

#[async_trait]
impl LineageStore for ObjectLineageStore {
    async fn list_runs(&self) -> Result<Vec<RunSummary>, LineageError> {
        let history = self.key(HISTORY_DIR)?;
        let mut runs = Vec::new();
        for meta in self.list(&history).await? {
            // Only `.history/<one segment>`: skips directory markers and nested objects.
            if meta.location.prefix_match(&history).map(Iterator::count) == Some(1) {
                let bytes = self.read(&meta.location).await?;
                runs.push(parse_history(&bytes, meta.location.as_ref())?);
            }
        }
        runs.sort_by(|a, b| (&a.timestamp, &a.lid).cmp(&(&b.timestamp, &b.lid)));
        Ok(runs)
    }

    async fn get(&self, lid: &Lid) -> Result<Record, LineageError> {
        let location = self.key(&format!("{}/{RECORD_FILE}", lid.record_key()))?;
        match self.read(&location).await {
            Ok(bytes) => Record::parse(&bytes, location.as_ref()),
            Err(LineageError::Storage(object_store::Error::NotFound { .. })) => {
                Err(LineageError::NotFound {
                    target: lid.to_string(),
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn find(&self, query: &FindQuery) -> Result<Vec<Lid>, LineageError> {
        let candidates: Vec<(Result<Lid, LineageError>, Path)> = self
            .list(&self.prefix)
            .await?
            .into_iter()
            .filter_map(|meta| Some((self.record_lid(&meta.location)?, meta.location)))
            .collect();
        let matched: Vec<Option<Lid>> = stream::iter(candidates)
            .map(|(lid, location)| self.matching(lid, location, query))
            .buffer_unordered(16)
            .try_collect()
            .await?;
        let mut found: Vec<Lid> = matched.into_iter().flatten().collect();
        found.sort();
        found.dedup();
        Ok(found)
    }
}
