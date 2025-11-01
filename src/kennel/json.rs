use kennel_club::{creature::Creature, math::Vec2};
use serde::Serialize;

#[derive(Serialize)]
pub struct CreatureJson {
    id: String,
    url: String,
    display_name: String,
    radius: f64,
    position: Vec2,
    sprite_path: String,
}

impl From<&Creature> for CreatureJson {
    fn from(creature: &Creature) -> Self {
        let sprite_path = format!("/api/kennel-club/{}/img", creature.id);
        CreatureJson {
            id: creature.id.clone(),
            url: creature.url.clone(),
            display_name: creature.display_name.clone(),
            radius: creature.radius,
            position: creature.position,
            sprite_path,
        }
    }
}

impl CreatureJson {
    pub fn url(&self) -> String {
        self.url.clone()
    }
}
