use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};

pub trait AsCache<T> {
    fn expired_at(&self) -> u64;
    fn get(self) -> T;
}

pub trait AsCacheRef<T> {
    fn expired_at(&self) -> u64;
    fn get(&self) -> &T;
}

pub async fn cache_aside<T, R, F>(
    key: &str,
    cache_prefix: &str,
    redis_conn: &mut MultiplexedConnection,
    db_call: F,
) -> anyhow::Result<R>
where
    T: Serialize + DeserializeOwned + Clone + AsCache<R>,
    R: Serialize + DeserializeOwned + Clone,
    F: FnOnce() -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<R, anyhow::Error>> + Send>,
    >,
{
    let cache_key = format!("{cache_prefix}:{key}");

    // Attempt to get the data from the cache
    if let Some(cached_data) = redis_conn.get::<_, Option<String>>(&cache_key).await? {
        let cached_value = serde_json::from_str::<T>(&cached_data)?;
        if cached_value.expired_at() > chrono::Utc::now().timestamp() as u64 {
            return Ok(cached_value.get());
        } else {
            redis_conn.del::<_, u32>(&cache_key).await?;
        }
    }

    // Fetch the data using the provided closure/function
    let result = db_call().await?;

    // Cache the result
    let cache_data = serde_json::to_string(&result)?;
    redis_conn.set::<_, _, _>(&cache_key, cache_data).await?;

    Ok(result)
}
