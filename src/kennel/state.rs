use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use kennel_club::{ImageFormat, Kennel, Sprite};
use rand::{SeedableRng, rngs::StdRng, seq::IteratorRandom};

use crate::kennel::json::CreatureJson;

static IMAGE_WIDTH: u32 = 2048;
static IMAGE_HEIGHT: u32 = 2048;

type ImageResult = Option<Result<Vec<u8>, String>>;

pub struct State {
    kennel: Arc<Mutex<Kennel>>,
    is_shutdown: Arc<Mutex<bool>>,
    image_cache: Arc<Mutex<ImageResult>>,
    rng: Arc<Mutex<StdRng>>,
}

impl State {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut init_rng = StdRng::from_os_rng();
        let kennel = Kennel::load(dir, &mut init_rng)?;

        let kennel_rc = Arc::new(Mutex::new(kennel));
        let is_shutdown_rc = Arc::new(Mutex::new(false));
        let image_cache_rc = Arc::new(Mutex::new(None));

        let thread_kennel = kennel_rc.clone();
        let thread_is_shutdown = is_shutdown_rc.clone();
        let thread_image_cache = image_cache_rc.clone();

        thread::spawn(move || {
            let mut kennel_rng = rand::rng();
            loop {
                // graceful shutdown
                let is_shutdown = thread_is_shutdown
                    .lock()
                    .expect("Error reading kennel shutdown state");
                if *is_shutdown {
                    break;
                }
                drop(is_shutdown);

                thread::sleep(Duration::from_secs(1));

                // update kennel state
                let mut kennel = thread_kennel.lock().expect("Error reading kennel state");
                *kennel = kennel
                    .next(&mut kennel_rng)
                    .expect("Error generating next kennel state");
                drop(kennel);

                // clear image cache
                let mut image_cache = thread_image_cache
                    .lock()
                    .expect("Error reading kennel image cache");
                image_cache.take();
                drop(image_cache);
            }
        });

        Ok(State {
            kennel: kennel_rc,
            is_shutdown: is_shutdown_rc,
            image_cache: image_cache_rc,
            rng: Arc::new(Mutex::new(init_rng)),
        })
    }

    pub fn as_image(&self, format: ImageFormat) -> Result<Vec<u8>, String> {
        let mut image_cache = self.image_cache.lock().expect("Error reading image cache");
        let kennel_rc = self.kennel.clone();
        let cache_result = image_cache.get_or_insert_with(move || {
            let kennel = kennel_rc.lock().expect("Error reading kennel state");
            kennel.get_image(IMAGE_WIDTH, IMAGE_HEIGHT, format)
        });

        cache_result
            .as_ref()
            .map(|data| data.to_vec())
            .map_err(|message| message.clone())
    }

    pub fn as_json(&self) -> Vec<CreatureJson> {
        let kennel = self.kennel.lock().expect("Error reading kennel state");

        kennel
            .creatures()
            .into_iter()
            .map(CreatureJson::from)
            .collect()
    }

    pub fn get_creature(&self, id: &str) -> Option<CreatureJson> {
        let kennel = self.kennel.lock().expect("Error reading kennel state");

        kennel
            .creatures()
            .into_iter()
            .find(|creature| creature.id == id)
            .map(CreatureJson::from)
    }

    pub fn get_random_creature(&self) -> Option<CreatureJson> {
        let mut rng = self.rng.lock().expect("Error reading rng state");
        let kennel = self.kennel.lock().expect("Error reading kennel state");

        kennel
            .creatures()
            .into_iter()
            .choose(&mut rng)
            .map(CreatureJson::from)
    }

    pub fn get_sprite(&self, id: &str) -> Option<Sprite> {
        let kennel = self.kennel.lock().expect("Error reading kennel state");
        kennel.get_sprite(id).cloned()
    }

    pub fn shutdown(&self) {
        let mut is_shutdown = self.is_shutdown.lock().expect("Error shutting down kennel");

        *is_shutdown = true;
    }
}
