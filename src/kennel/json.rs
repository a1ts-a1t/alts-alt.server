use kennel_club::{creature::Creature, math::Vec2};
use serde::Serialize;

#[derive(Serialize)]
pub struct CreatureJson {
    id: String,
    radius: f64,
    position: Vec2,
    sprite_path: String,
}

impl From<&Creature> for CreatureJson {
    fn from(creature: &Creature) -> Self {
        let sprite_path = format!("/api/kennel-club/{}", creature.id);
        CreatureJson {
            id: creature.id.clone(),
            radius: creature.radius,
            position: creature.position,
            sprite_path,
        }
    }
}
