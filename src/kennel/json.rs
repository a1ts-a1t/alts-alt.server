use kennel_club::{
    Kennel, State,
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
        let sprite_sheet = &creature.sprite_sheet;
        let sprite_info = vec![
            (State::Idle, sprite_sheet.get_frame_count(State::Idle)),
            (State::Sleep, sprite_sheet.get_frame_count(State::Sleep)),
            (State::East, sprite_sheet.get_frame_count(State::East)),
            (
                State::Northeast,
                sprite_sheet.get_frame_count(State::Northeast),
            ),
            (State::North, sprite_sheet.get_frame_count(State::North)),
            (
                State::Northwest,
                sprite_sheet.get_frame_count(State::Northwest),
            ),
            (State::West, sprite_sheet.get_frame_count(State::West)),
            (
                State::Southwest,
                sprite_sheet.get_frame_count(State::Southwest),
            ),
            (State::South, sprite_sheet.get_frame_count(State::South)),
            (
                State::Southeast,
                sprite_sheet.get_frame_count(State::Southeast),
            ),
        ];

        let sprite_paths: Vec<String> = sprite_info
            .into_iter()
            .flat_map(|(state, len)| {
                (0_usize..len).map(move |frame| build_sprite_path(&creature.id, state, frame))
            })
            .collect();

        let sprite_path =
            build_sprite_path(&creature.id, creature.sprite_state, creature.sprite_frame);

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

fn build_sprite_path(id: &str, state: State, frame: usize) -> String {
    format!("/api/kennel-club/{}/img/{}/{}", id, state, frame)
}
