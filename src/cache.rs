use std::{collections::HashMap, hash::Hash, time::{Duration, SystemTime}};

static DEFAULT_TTL: Duration = Duration::from_secs(60);

pub struct CacheEntry<T> {
    item: T,
    expiration: SystemTime,
}

struct Cache<K: Eq + Hash, V> {
    map: HashMap<K, CacheEntry<V>>,
    ttl: Duration,
}

impl <K: Eq + Hash, V> Cache<K, V> {
    pub fn new(ttl: Duration) -> Cache<K, V> {
        Cache {
            map: HashMap::new(),
            ttl: ttl,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.map.get(key)
            .filter(|cache_item| {
                cache_item.expiration > SystemTime::now()
            })
            .map(|cache_item| &cache_item.item)
    }

    pub fn put(&mut self, key: K, value: V) -> () {
        self.put_with_ttl(key, value, self.ttl);
    }

    pub fn put_with_ttl(&mut self, key: K, value: V, ttl: Duration) -> () {
        let expiration = SystemTime::now() + ttl;
        let cache_entry = CacheEntry {
            item: value,
            expiration,
        };
        self.map.insert(key, cache_entry);
    }

    pub fn clean_up(&mut self) -> () {
        self.map.retain(|_, cache_entry| {
            cache_entry.expiration > SystemTime::now()
        });
    }
}

impl <K: Eq + Hash, V> Default for Cache<K, V> {
    fn default() -> Self {
        Cache::new(DEFAULT_TTL)
    }
}

