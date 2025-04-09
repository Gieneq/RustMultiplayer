use super::math::{
    Rect2F,
    Vector2F
};

use rand::seq::IndexedRandom;

pub const TILE_SIZE: f32 = 5.0;
pub const ENTITY_SIZE: Vector2F = Vector2F {
    x: TILE_SIZE - 0.2,
    y: TILE_SIZE - 0.2
};

#[derive(Debug)]
pub enum WorldError {
    EntityNotExist,
    EntityCannotMoveThere,
}

#[derive(Debug)]
pub struct World {
    new_entity_id: EntityId,
    entities: Vec<Entity>,
}

#[derive(Debug, PartialEq)]
pub enum EntityState {
    Idle,
    Moving {
        from_position: Vector2F,
        destination: Vector2F,
    },
}

#[derive(Debug)]
pub struct NpcController {
    spawnpoint: Vector2F,
    roaming_range: Option<f32>,
    change_destination_counter: u32,
}

#[derive(Debug)]
pub struct PlayerController;

#[derive(Debug)]
pub enum EntityController {
    Npc(NpcController),
    Player(PlayerController),
}

pub type EntityId = u32;

#[derive(Debug)]
pub struct EntityStats {
    movement_speed: f32,
}

#[derive(Debug)]
pub struct Entity {
    pub id: u32,
    pub name: String,
    pub position: Vector2F,
    pub color: [u8; 3],
    pub size: Vector2F,
    state: EntityState,
    stats: EntityStats,
    controller: EntityController,
}

const PLAYER_MOVEMENT_SPEED: f32 = 0.9;
const NPC_MOVEMENT_SPEED: f32 = 0.3;
const NPC_DIRECTION_SELECTION_TICKS_RANGE: std::ops::Range<u32> = 5..40;

impl World {
    pub fn get_grid_aligned_position(pos: &Vector2F) -> Vector2F {
        fn align_coord(coord: f32) -> f32 {
            (coord / TILE_SIZE).floor() * TILE_SIZE
        }

        Vector2F::new(  
            align_coord(pos.x),
            align_coord(pos.y)
        )
    }

    pub fn new() -> Self {
        log::info!("World created");
        Self {
            new_entity_id: 0,
            entities: vec![],
        }
    }

    pub fn create_entity_player<S: AsRef<str>>(&mut self, name: S, intial_position: Vector2F, size: Vector2F) -> EntityId {
        let intial_position = Self::get_grid_aligned_position(&intial_position);
        let colors = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 0, 255],
            [0, 255, 255],
            [255, 255, 0],
        ];
        let mut rng = rand::rng();
        let color = *colors.choose(&mut rng).unwrap();
        self.create_entity(
            name, 
            intial_position, 
            size,
            color,
            EntityStats {
                movement_speed: PLAYER_MOVEMENT_SPEED
            }, 
            EntityController::Player(PlayerController)
        )
    }

    pub fn create_entity_npc<S: AsRef<str>>(&mut self, name: S, intial_position: Vector2F, size: Vector2F) -> EntityId {
        let intial_position = Self::get_grid_aligned_position(&intial_position);
        let channel = rand::random_range(35..150);
        let color = [channel, channel, channel];
        self.create_entity(
            name, 
            intial_position, 
            size,
            color,
            EntityStats {
                movement_speed: NPC_MOVEMENT_SPEED
            }, 
            EntityController::Npc(NpcController {
                spawnpoint: intial_position,
                roaming_range: Some(TILE_SIZE * 2.5),
                change_destination_counter: rand::random_range(NPC_DIRECTION_SELECTION_TICKS_RANGE)
            })
        )
    }

    pub fn create_entity<S: AsRef<str>>(&mut self, name: S, intial_position: Vector2F, size: Vector2F, color: [u8; 3], stats: EntityStats, controller: EntityController) -> EntityId {
        let intial_position = Self::get_grid_aligned_position(&intial_position);
        let new_id = self.new_entity_id;
        self.new_entity_id += 1;

        let entity = Entity { 
            id: new_id, 
            name: name.as_ref().to_string(),
            position: intial_position,
            size,
            color,
            state: EntityState::Idle,
            stats,
            controller
        };

        self.entities.push(entity);
        new_id
    }

    pub fn remove_entity(&mut self, entity_id: EntityId) -> Result<(), WorldError> {
        let position = self.entities.iter()
            .position(|e| e.id == entity_id)
            .ok_or(WorldError::EntityNotExist)?;
        let _ = self.entities.remove(position);
        Ok(())
    }

    pub fn get_entity_by_id(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == entity_id)
    }

    pub fn get_entity_by_id_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == entity_id)
    }

    pub fn is_tile_occupied(&self, tile_position: &Vector2F) -> bool {
        let checked_tile = Rect2F::new(tile_position.x, tile_position.y, TILE_SIZE, TILE_SIZE);
        for entity in self.entities.iter() {
            let is_colliding = match entity.state {
                EntityState::Idle => checked_tile.contains(&entity.position),
                EntityState::Moving { from_position, destination } => {
                    checked_tile.contains(&from_position) || checked_tile.contains(&destination)
                },
            };

            if is_colliding {
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self) {
        log::trace!("World tick");

        // TODO Do it better
        // BUG 2 entities can select the same destination this way
        let occupied_positions: Vec<_> = self
        .entities
        .iter()
        .map(|e| match &e.state {
            EntityState::Moving { destination, from_position } => vec![*destination, *from_position],
            EntityState::Idle => vec![e.position],
        })
        .flatten()
        .collect();

        self.entities.iter_mut().for_each(|e| {
            log::trace!(" - {e:?}");

            if let EntityState::Moving { from_position, destination } = e.state {
                // Interpolate movement
                // Destination was checked when entity was idle -> no need to check
                let direction = (destination - from_position).normal();
                let previous_location_to_destination = destination - e.position;
                e.position += direction * e.stats.movement_speed;    
                let new_location_to_destination = destination - e.position;        
                
                // Check if destination was reached, by checking change of dot product
                const DOT_PRODCT_CLOSNESS_MARGIN: f32 = 0.01;
                let transition_closness = previous_location_to_destination.dot(new_location_to_destination);    
                let destination_was_reached = transition_closness <= DOT_PRODCT_CLOSNESS_MARGIN;

                log::trace!("   {} moving, now in {}, closeness: {}", e.name, e.position, transition_closness);

                // Align to destination, Change state to Idle and reset counter
                if destination_was_reached {
                    log::debug!("   {} reached destination {} go IDLE", e.name, destination);
                    e.state = EntityState::Idle;
                    if let EntityController::Npc(npc_controller) = &mut e.controller {
                        npc_controller.change_destination_counter = rand::random_range(NPC_DIRECTION_SELECTION_TICKS_RANGE);
                    }
                    e.position = destination;
                }
            }

            match &mut e.controller {
                EntityController::Npc(npc_controller) => {
                    // Check if is capable of roaming
                    if let Some(range) = npc_controller.roaming_range {

                        // Every `xx` try selecting new destination
                        // If destination is not valid (out of range, occupied or reserved)
                        // then try next time.
                        if e.state == EntityState::Idle {
                            // Count down, at counting exhaustion try selecting new destination
                            if npc_controller.change_destination_counter > 0 {
                                log::debug!("   {} counting in IDLE {}...", e.name, npc_controller.change_destination_counter);
                                npc_controller.change_destination_counter -= 1;
                            } else {
                                // Triggered -> try selecting new destination
                                let directions = [
                                    Vector2F::new(1.0, 0.0),
                                    Vector2F::new(-1.0, 0.0),
                                    Vector2F::new(0.0, 1.0),
                                    Vector2F::new(0.0, -1.0),
                                ];

                                let random_direction = directions.choose(&mut rand::rng()).unwrap();

                                let destination_position = e.position + (*random_direction * TILE_SIZE);

                                if (destination_position - npc_controller.spawnpoint).length_squared() > range.powi(2) {
                                    log::trace!("   Tile {destination_position} out of range!");
                                    return;
                                }

                                if !occupied_positions.contains(&destination_position) {
                                    log::info!("   {} Setting new destination from {} -to-> {} go MOVING!", 
                                        e.name, e.position, destination_position
                                    );
                                    e.state = EntityState::Moving {
                                        from_position: e.position,
                                        destination: destination_position
                                    };
                                } else {
                                    log::info!("   Tile {destination_position} already occupied!");
                                }
                            }
                        }
                    }
                },
                EntityController::Player(_player_controller) => { },
            }
        });
    }
    
    pub fn iter_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    pub fn try_start_move_entity_to(&mut self, entity_id: EntityId, next_position: Vector2F) -> Result<(), WorldError> {
        if self.is_tile_occupied(&next_position) {
            Err(WorldError::EntityCannotMoveThere)
        } else {
            let entity = self.get_entity_by_id_mut(entity_id).ok_or(WorldError::EntityNotExist)?;
            
            entity.state = EntityState::Moving {
                from_position: entity.position,
                destination: next_position
            };
            Ok(())
        }
    }

}

impl Entity {
    pub fn is_player(&self) -> bool {
        matches!(self.controller, EntityController::Player(_))
    }

    pub fn is_moving(&self) -> bool {
        matches!(self.state, EntityState::Moving { from_position: _, destination: _ })
    }
}

#[test]
fn test_world_creation() {
    let world = World::new();
    assert_eq!(world.new_entity_id, 0);
}

#[test]
fn test_world_entity_creation_should_increase_entities_count() {
    let mut world = World::new();
    assert_eq!(world.new_entity_id, 0);

    let new_entity_id = world.create_entity_npc("Bob", Vector2F::new(1.0, 2.0), Vector2F::new(1.0, 1.0));
    assert_eq!(new_entity_id, 0);

    assert_eq!(world.new_entity_id, 1);
}

#[test]
fn test_world_entity_access() {
    let entity_name = "Bob";
    let entity_position = Vector2F::new(1.0, 2.0);

    let mut world = World::new();
    let new_entity_id = world.create_entity_npc(entity_name, entity_position, Vector2F::new(1.0, 1.0));

    let entity = world.get_entity_by_id(new_entity_id).unwrap();
    assert_eq!(entity.name, entity_name);
    assert_eq!(entity.position, entity_position);
    assert_eq!(entity.state, EntityState::Idle);
}

#[test]
fn test_world_entity_translate() {
    let entity_initial_position = Vector2F::new(1.0, 2.0);
    let translation = Vector2F::new(100.0, 500.0);

    let mut world = World::new();
    let new_entity_id = world.create_entity_npc("Bob", entity_initial_position, Vector2F::new(1.0, 1.0));

    let entity = world.get_entity_by_id_mut(new_entity_id).unwrap();
    entity.position += translation;
    assert_eq!(entity.position, entity_initial_position + translation);
}

#[test]
fn test_coords_positive_alignment() {
    let x_tiles_count: f32 = 0.0;
    let y_tiles_count: f32 = 2.0;

    let v1 = Vector2F::new((x_tiles_count + 0.1) * TILE_SIZE, (y_tiles_count + 0.7) * TILE_SIZE);
    let v2 = World::get_grid_aligned_position(&v1);
    let v2_expected = Vector2F::new(x_tiles_count * TILE_SIZE, y_tiles_count * TILE_SIZE);

    assert_eq!(v2, v2_expected, "v1={v1:?}");
}

#[test]
fn test_coords_negative_alignment() {
    let x_tiles_count: f32 = 0.0;
    let y_tiles_count: f32 = -3.0;

    let v1 = Vector2F::new((x_tiles_count - 0.1) * TILE_SIZE, (y_tiles_count - 0.7) * TILE_SIZE);
    let v2 = World::get_grid_aligned_position(&v1);
    let v2_expected = Vector2F::new((x_tiles_count - 1.0) * TILE_SIZE, (y_tiles_count - 1.0) * TILE_SIZE);

    assert_eq!(v2, v2_expected, "v1={v1:?}");
}