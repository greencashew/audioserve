use super::{error::Error, CacheInnerType, Result};
use std::fs;
use tokio;
use tokio::task::spawn_blocking;

impl From<tokio::task::JoinError> for Error {
    fn from(_f: tokio::task::JoinError) -> Self {
        Error::Executor
    }
}

fn invert<T>(x: Option<Result<T>>) -> Result<Option<T>> {
    x.map_or(Ok(None), |v| v.map(Some))
}

pub(crate) async fn get_asynch_file(
    key: String,
    cache: CacheInnerType,
) -> Result<Option<tokio::fs::File>> {
    let r = spawn_blocking(move || {
        let mut c = cache.write().expect("Cannot lock cache");
        c.get(key.clone())
            .map(|f| f.map(|f| tokio::fs::File::from_std(f)))
    })
    .await?;
    invert(r)
}

pub(crate) async fn get_asynch_file2(
    key: String,
    cache: CacheInnerType,
) -> Result<Option<(tokio::fs::File, std::path::PathBuf)>> {
    let r = spawn_blocking(move || {
        let mut c = cache.write().expect("Cannot lock cache");
        c.get2(key.clone())
            .map(|f| f.map(|(f, path)| (tokio::fs::File::from_std(f), path)))
    })
    .await?;
    invert(r)
}

pub(crate) async fn save_index_asynch(cache: CacheInnerType) -> Result<()> {
    spawn_blocking(move || {
        let cache = cache.write().unwrap();
        cache.save_index()
    })
    .await?
}

pub struct Finisher {
    pub(crate) cache: CacheInnerType,
    pub(crate) key: String,
    pub(crate) file: fs::File,
}

impl Finisher {
    pub async fn commit(mut self) -> Result<()> {
        spawn_blocking(move || {
            let mut c = self.cache.write().expect("Cannot lock cache");
            c.finish(self.key, &mut self.file)
        })
        .await?
    }

    pub async fn roll_back(self) -> Result<()> {
        spawn_blocking(move || super::cleanup(&self.cache, self.key))
            .await
            .map_err(From::from)
    }
}

pub(crate) async fn get_asynch_writable(
    key: String,
    cache: CacheInnerType,
) -> Result<(tokio::fs::File, Finisher)> {
    spawn_blocking(move || {
        let mut c = cache.write().expect("Cannot lock cache");
        c.add(key.clone())
            .and_then(|f| f.try_clone().map_err(|e| e.into()).map(|f2| (f, f2)))
            .map(|(f, f2)| {
                (
                    tokio::fs::File::from_std(f),
                    Finisher {
                        cache: cache.clone(),
                        key: key,
                        file: f2,
                    },
                )
            })
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cache;
    use tempfile::tempdir;

    const MY_KEY: &str = "muj_test_1";
    const MSG: &str = "Hello there you lonely bastard";

    async fn cache_rw(c: Cache) -> Result<()> {
        use tokio::prelude::*;
        let (mut f, fin) = c.add_async(String::from(MY_KEY)).await?;
        f.write_all(MSG.as_bytes()).await?;
        fin.commit().await?;
        match c.get_async(MY_KEY).await? {
            None => panic!("cache file not found"),
            Some(mut f) => {
                let mut v = Vec::new();
                f.read_to_end(&mut v).await?;
                let s = std::str::from_utf8(&v).unwrap();
                assert_eq!(MSG, s);
                info!("ALL DONE");
            }
        }

        Ok(())
    }

    #[test]
    fn test_async() {
        use std::io::Read;

        env_logger::try_init().ok();
        let temp_dir = tempdir().unwrap();
        let c = Cache::new(temp_dir.path(), 10000, 10).unwrap();
        let c2 = c.clone();
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(cache_rw(c)).unwrap();
        c2.get(MY_KEY)
            .unwrap()
            .map(|mut f| {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                assert_eq!(MSG, s);
                ()
            })
            .unwrap()
    }
}
