use crate::types::AsyncFn;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub struct CacheConfig {
    pub max_age: Duration,
}

pub fn create_cached_fn<F, C>(func: F, config: CacheConfig) -> impl Fn() -> C + Send + Sync + 'static
where
    F: Fn() -> C + Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
{
    let refresh_time_mutex = Arc::new(Mutex::new(SystemTime::now()));
    let cached_data_mutex: Arc<Mutex<Option<C>>> = Arc::new(Mutex::new(None));

    move || {
        let refresh_time = Arc::clone(&refresh_time_mutex);
        let mut refresh_time = refresh_time.lock().unwrap();

        let cached_data = Arc::clone(&cached_data_mutex);
        let mut cached_data = cached_data.lock().unwrap();

        let is_cache_fresh = SystemTime::now()
            .duration_since(*refresh_time)
            .map(|duration_since| duration_since.le(&config.max_age))
            .unwrap_or(false);

        let fresh_data = (*cached_data).as_ref().filter(|_| is_cache_fresh);

        match fresh_data {
            Some(data) => data.clone(),
            None => {
                let new_data = func();
                *refresh_time = SystemTime::now();
                *cached_data = Some(new_data.clone());
                new_data
            },
        }
    }
}

pub fn create_cached_async_fn<F, Fut, C>(func: F, config: CacheConfig)
    -> impl AsyncFn<Future = Pin<Box<impl Future<Output = C>>>, Out = C> + Clone + Send + Sync + 'static
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = C> + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static
{
    let refresh_time_mutex = Arc::new(tokio::sync::Mutex::new(SystemTime::now()));
    let cached_data_mutex: Arc<tokio::sync::Mutex<Option<C>>> = Arc::new(tokio::sync::Mutex::new(None));

    move || {
        let refresh_time_clone = Arc::clone(&refresh_time_mutex);
        let cached_data_clone = Arc::clone(&cached_data_mutex);
        let func_clone = func.clone();

        Box::pin(async move {
            let mut refresh_time = refresh_time_clone.lock().await;
            let mut cached_data = cached_data_clone.lock().await;

            let is_cache_fresh = SystemTime::now()
                .duration_since(*refresh_time)
                .map(|duration_since| duration_since.le(&config.max_age))
                .unwrap_or(false);

            let fresh_data = (*cached_data).as_ref().filter(|_| is_cache_fresh);

            match fresh_data {
                Some(data) => data.clone(),
                None => {
                    let new_data = func_clone().await;
                    *refresh_time = SystemTime::now();
                    *cached_data = Some(new_data.clone());
                    new_data
                },
            }
        })
    }
}

