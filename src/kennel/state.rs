use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use kennel_club::{ImageFormat, Kennel, Sprite};
use rand::{SeedableRng, rngs::StdRng, seq::IteratorRandom};
use rocket::{
    futures::lock::Mutex,
    tokio::{
        self,
        sync::mpsc::{self, Receiver, Sender},
        time::sleep,
    },
};
use uuid::Uuid;

use crate::kennel::json::{CreatureJson, KennelJson};

static IMAGE_WIDTH: u32 = 2048;
static IMAGE_HEIGHT: u32 = 2048;

type ImageResult = Option<Result<Vec<u8>, String>>;

fn safe_rng() -> StdRng {
    let mut rng = rand::rng();
    StdRng::from_rng(&mut rng)
}

pub struct State {
    kennel: Arc<Mutex<Kennel>>,
    is_shutdown: Arc<Mutex<bool>>,
    image_cache: Arc<Mutex<ImageResult>>,
    subscribers: Arc<Mutex<HashMap<Uuid, Sender<KennelJson>>>>,
}

impl State {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut init_rng = safe_rng();
        let kennel = Kennel::load(dir, &mut init_rng)?;
        let subscribers: HashMap<Uuid, Sender<KennelJson>> = HashMap::new();

        let kennel_rc = Arc::new(Mutex::new(kennel));
        let is_shutdown_rc = Arc::new(Mutex::new(false));
        let image_cache_rc = Arc::new(Mutex::new(None));
        let subscribers_rc = Arc::new(Mutex::new(subscribers));

        let thread_kennel = kennel_rc.clone();
        let thread_is_shutdown = is_shutdown_rc.clone();
        let thread_image_cache = image_cache_rc.clone();
        let thread_subscribers = subscribers_rc.clone();

        tokio::spawn(async move {
            let mut kennel_rng = safe_rng();
            loop {
                // graceful shutdown
                let is_shutdown = thread_is_shutdown.lock().await;
                if *is_shutdown {
                    break;
                }
                drop(is_shutdown);

                sleep(Duration::from_secs(1)).await;

                // update kennel state
                let mut kennel = thread_kennel.lock().await;
                let next_kennel = kennel
                    .next(&mut kennel_rng)
                    .expect("Error generating next kennel state");

                let subscribers = thread_subscribers.lock().await;

                let kennel_json = KennelJson::from(&next_kennel);
                for subscriber in subscribers.values() {
                    let _ = subscriber.send(kennel_json.clone()).await;
                }
                drop(subscribers);

                *kennel = next_kennel;
                drop(kennel);

                // clear image cache
                let mut image_cache = thread_image_cache.lock().await;
                image_cache.take();
                drop(image_cache);
            }
        });

        Ok(State {
            kennel: kennel_rc,
            is_shutdown: is_shutdown_rc,
            image_cache: image_cache_rc,
            subscribers: subscribers_rc,
        })
    }

    pub async fn as_image(&self, format: ImageFormat) -> Result<Vec<u8>, String> {
        let mut image_cache = self.image_cache.lock().await;
        let kennel = self.kennel.lock().await;
        let cache_result = image_cache
            .get_or_insert_with(move || kennel.get_image(IMAGE_WIDTH, IMAGE_HEIGHT, format));

        cache_result
            .as_ref()
            .map(|data| data.to_vec())
            .map_err(|message| message.clone())
    }

    pub async fn as_json(&self) -> Vec<CreatureJson> {
        let kennel = self.kennel.lock().await;

        kennel
            .creatures()
            .into_iter()
            .map(CreatureJson::from)
            .collect()
    }

    pub async fn get_creature(&self, id: &str) -> Option<CreatureJson> {
        let kennel = self.kennel.lock().await;

        kennel
            .creatures()
            .into_iter()
            .find(|creature| creature.id == id)
            .map(CreatureJson::from)
    }

    pub async fn get_random_creature(&self) -> Option<CreatureJson> {
        let mut rng = safe_rng();
        let kennel = self.kennel.lock().await;

        kennel
            .creatures()
            .into_iter()
            .choose(&mut rng)
            .map(CreatureJson::from)
    }

    pub async fn get_sprite(&self, id: &str) -> Option<Sprite> {
        let kennel = self.kennel.lock().await;
        kennel.get_sprite(id).cloned()
    }

    pub async fn subscribe(&self) -> (Uuid, Receiver<KennelJson>) {
        let mut subscribers = self.subscribers.lock().await;
        let (tx, rx) = mpsc::channel(1);
        let id = Uuid::new_v4();

        subscribers.insert(id, tx);
        (id, rx)
    }

    pub async fn unsubscribe(&self, id: &Uuid) {
        let mut subscribers = self.subscribers.lock().await;
        subscribers.remove(id);
    }

    pub async fn shutdown(&self) {
        let mut is_shutdown = self.is_shutdown.lock().await;
        *is_shutdown = true;
    }
}
