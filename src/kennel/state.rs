use std::{
    collections::HashMap, path::Path, sync::{Arc, Mutex}, thread, time::Duration
};

use kennel_club::{Kennel, ImageFormat};

use crate::kennel::json::CreatureJson;

static IMAGE_WIDTH: u32 = 2048;
static IMAGE_HEIGHT: u32 = 2048;

type ImageResultCache = HashMap<ImageFormat, Result<Vec<u8>, String>>;

pub struct State {
    kennel: Arc<Mutex<Kennel>>,
    is_shutdown: Arc<Mutex<bool>>,
    image_cache: Arc<Mutex<ImageResultCache>>,
}

impl State {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut init_rng = rand::rng();
        let kennel = Kennel::load(dir, &mut init_rng)?;

        let kennel_rc = Arc::new(Mutex::new(kennel));
        let is_shutdown_rc = Arc::new(Mutex::new(false));
        let image_cache_rc = Arc::new(Mutex::new(HashMap::new()));

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
                let mut image_cache = thread_image_cache.lock().expect("Error reading kennel image cache");
                image_cache.clear();
                drop(image_cache);
            }
        });

        Ok(State {
            kennel: kennel_rc,
            is_shutdown: is_shutdown_rc,
            image_cache: image_cache_rc,
        })
    }

    pub fn as_image(&self, format: ImageFormat) -> Result<Vec<u8>, String> {
        let mut image_cache = self.image_cache.lock().expect("Error reading image cache");
        let kennel_rc = self.kennel.clone();
        let cache_result = image_cache.entry(format).or_insert_with(move || {
            let kennel = kennel_rc.lock().expect("Error reading kennel state");
            kennel.get_image(IMAGE_WIDTH, IMAGE_HEIGHT, format)
        });

        cache_result.as_ref()
            .map(|data| data.to_vec())
            .map_err(|message| message.clone())
    }

    pub fn as_json(&self) -> Result<Vec<CreatureJson>, serde_json::Error> {
        let kennel = self.kennel.lock().expect("Error reading kennel state");

        Ok(kennel
            .creatures()
            .into_iter()
            .map(CreatureJson::from)
            .collect())
    }

    pub fn shutdown(&self) {
        let mut is_shutdown = self.is_shutdown.lock().expect("Error shutting down kennel");

        *is_shutdown = true;
    }
}
