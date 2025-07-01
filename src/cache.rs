use std::{
    collections::HashMap,
    hash::Hash,
    sync::Mutex,
    time::{Duration, SystemTime},
};

static DEFAULT_TTL: Duration = Duration::from_secs(60);

pub struct CacheEntry<T> {
    item: T,
    expiration: SystemTime,
}

pub struct Cache<K: Eq + Hash, V: Clone> {
    map: Mutex<HashMap<K, CacheEntry<V>>>,
    ttl: Duration,
}

impl<K: Eq + Hash, V: Clone> Cache<K, V> {
    pub fn new(ttl: Duration) -> Cache<K, V> {
        Cache {
            map: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let map = self.map.lock().expect("Lock cache data");
        map.get(key)
            .filter(|cache_item| cache_item.expiration > SystemTime::now())
            .map(|cache_item| cache_item.item.clone())
    }

    pub fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.ttl);
    }

    pub fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut map = self.map.lock().expect("Lock cache data");
        let expiration = SystemTime::now() + ttl;
        let cache_entry = CacheEntry {
            item: value,
            expiration,
        };
        map.insert(key, cache_entry);
    }

    // TODO: clean up method if/when it's relevant
}

impl<K: Eq + Hash, V: Clone> Default for Cache<K, V> {
    fn default() -> Self {
        Cache::new(DEFAULT_TTL)
    }
}
