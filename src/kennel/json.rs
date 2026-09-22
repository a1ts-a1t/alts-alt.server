use kennel_club::{
    Kennel,
    creature::{self, Creature},
    math::Vec2,
};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct CreatureJson {
    id: String,
    url: String,
    display_name: String,
    radius: f64,
    position: Vec2,
    state: creature::State,
    sprite_path: String,
    sprite_paths: Vec<String>,
}

impl From<&Creature> for CreatureJson {
    fn from(creature: &Creature) -> Self {
        let sprite_paths: Vec<String> = creature
            .sprite_sheet
            .get_info()
            .into_iter()
            .flat_map(|(state, len)| {
                (0_usize..len).map(move |frame| {
                    format!("/api/kennel-club/{}/img/{}/{}", creature.id, state, frame)
                })
            })
            .collect();

        let sprite_path = format!(
            "/api/kennel-club/{}/img/{}/{}",
            creature.id, creature.sprite_state, creature.sprite_state_duration
        );

        CreatureJson {
            id: creature.id.clone(),
            url: creature.url.clone(),
            display_name: creature.display_name.clone(),
            radius: creature.radius,
            position: creature.position,
            state: creature.creature_state.clone(),
            sprite_path,
            sprite_paths,
        }
    }
}

impl CreatureJson {
    pub fn url(&self) -> String {
        self.url.clone()
    }
}

#[derive(Serialize, Clone)]
#[serde(transparent)]
pub struct KennelJson {
    creatures: Vec<CreatureJson>,
}

impl From<&Kennel> for KennelJson {
    fn from(kennel: &Kennel) -> Self {
        Self {
            creatures: kennel
                .creatures()
                .into_iter()
                .map(CreatureJson::from)
                .collect(),
        }
    }
}
