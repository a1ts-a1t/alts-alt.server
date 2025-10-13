use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use kennel_club::Kennel;

use crate::kennel::json::CreatureJson;

pub struct State {
    kennel: Arc<Mutex<Kennel>>,
    is_shutdown: Arc<Mutex<bool>>,
}

impl State {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut init_rng = rand::rng();
        let kennel = Kennel::load(dir, &mut init_rng)?;

        let kennel_rc = Arc::new(Mutex::new(kennel));
        let is_shutdown_rc = Arc::new(Mutex::new(false));

        let thread_kennel = kennel_rc.clone();
        let thread_is_shutdown = is_shutdown_rc.clone();
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
                let mut kennel = thread_kennel.lock().expect("Error reading kennel state");
                *kennel = kennel
                    .next(&mut kennel_rng)
                    .expect("Error generating next kennel state");
                drop(kennel);
            }
        });

        Ok(State {
            kennel: kennel_rc,
            is_shutdown: is_shutdown_rc,
        })
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
