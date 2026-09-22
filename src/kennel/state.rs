use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use kennel_club::{ImageFormat, Kennel, Sprite, State as SpriteState};
use rand::{SeedableRng, rngs::StdRng, seq::IteratorRandom};
use tokio::sync::Mutex;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::sleep,
};
use uuid::Uuid;

use crate::kennel::json::{CreatureJson, KennelJson};

static IMAGE_WIDTH: u32 = 2048;
static IMAGE_HEIGHT: u32 = 2048;
static FRAME_DURATION: Duration = Duration::from_millis(1000 / 12); // 12 fps

type ImageResult = Option<Result<Vec<u8>, String>>;

fn safe_rng() -> StdRng {
    let mut rng = rand::rng();
    StdRng::from_rng(&mut rng)
}

pub struct KennelState {
    kennel: Result<Arc<Mutex<Kennel>>, String>,
    is_shutdown: Arc<Mutex<bool>>,
    image_cache: Arc<Mutex<ImageResult>>,
    subscribers: Arc<Mutex<HashMap<Uuid, Sender<KennelJson>>>>,
}

impl KennelState {
    pub fn load(dir: &Path) -> Self {
        let mut init_rng = safe_rng();
        let kennel = Kennel::load(dir, &mut init_rng).map(|kennel| Arc::new(Mutex::new(kennel)));

        let state = KennelState {
            kennel,
            is_shutdown: Arc::new(Mutex::new(false)),
            image_cache: Arc::new(Mutex::new(None)),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        };
        state.spawn_updates();

        state
    }
    pub fn availability(&self) -> Result<(), String> {
        self.loaded().map(|_| ())
    }

    fn loaded(&self) -> Result<&Arc<Mutex<Kennel>>, String> {
        self.kennel.as_ref().map_err(|message| message.clone())
    }

    fn spawn_updates(&self) {
        let Some(thread_kennel) = self.kennel.as_ref().ok().cloned() else {
            return;
        };
        let thread_is_shutdown = self.is_shutdown.clone();
        let thread_image_cache = self.image_cache.clone();
        let thread_subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut kennel_rng = safe_rng();
            loop {
                // graceful shutdown
                let is_shutdown = thread_is_shutdown.lock().await;
                if *is_shutdown {
                    break;
                }
                drop(is_shutdown);

                sleep(FRAME_DURATION).await;

                // update kennel state
                let mut kennel = thread_kennel.lock().await;
                let next_kennel = kennel
                    .next(&mut kennel_rng)
                    .expect("Error generating next kennel state");

                let subscribers = thread_subscribers.lock().await;

                let kennel_json = KennelJson::from(&next_kennel);
                for subscriber in subscribers.values() {
                    let _ = subscriber.try_send(kennel_json.clone());
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
    }

    pub async fn as_image(&self, format: ImageFormat) -> Result<Vec<u8>, String> {
        let mut image_cache = self.image_cache.lock().await;
        let kennel = self.loaded()?.lock().await;

        let cache_result = image_cache
            .get_or_insert_with(move || kennel.get_image(IMAGE_WIDTH, IMAGE_HEIGHT, format));

        cache_result
            .as_ref()
            .map(|data| data.to_vec())
            .map_err(|message| message.clone())
    }

    pub async fn as_json(&self) -> Result<Vec<CreatureJson>, String> {
        let kennel = self.loaded()?.lock().await;

        Ok(kennel
            .creatures()
            .into_iter()
            .map(CreatureJson::from)
            .collect())
    }

    pub async fn get_creature(&self, id: &str) -> Result<Option<CreatureJson>, String> {
        let kennel = self.loaded()?.lock().await;

        Ok(kennel
            .creatures()
            .into_iter()
            .find(|creature| creature.id == id)
            .map(CreatureJson::from))
    }

    pub async fn get_random_creature(&self) -> Result<Option<CreatureJson>, String> {
        let mut rng = safe_rng();
        let kennel = self.loaded()?.lock().await;

        Ok(kennel
            .creatures()
            .into_iter()
            .choose(&mut rng)
            .map(CreatureJson::from))
    }

    pub async fn get_sprite(&self, id: &str) -> Result<Option<Sprite>, String> {
        let kennel = self.loaded()?.lock().await;
        Ok(kennel.get_sprite(id).cloned())
    }

    pub async fn get_sprite_by(
        &self,
        id: &str,
        sprite_state: &str,
        frame: &usize,
    ) -> Result<Option<Sprite>, String> {
        let kennel = self.loaded()?.lock().await;
        Ok(SpriteState::try_from(sprite_state)
            .ok()
            .and_then(|s| kennel.get_sprite_by(id, &s, frame).cloned()))
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
