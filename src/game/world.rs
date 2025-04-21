use crate::game::math::Vector2I;

use super::math::{
    Rect2F,
    Vector2F
};

use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

pub const TILE_SIZE: f32 = 5.0;
pub const ENTITY_SIZE: Vector2F = Vector2F {
    x: TILE_SIZE - 0.2,
    y: TILE_SIZE - 0.2
};

pub fn get_tiled_value(v: i32) -> f32 {
    v as f32 * TILE_SIZE
}

pub fn get_tiled_vec(x: i32, y: i32) -> Vector2F {
    Vector2F {
        x: get_tiled_value(x),
        y: get_tiled_value(y)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("EntityNotExist")]
    EntityNotExist,

    #[error("EntityCannotMoveThere")]
    EntityCannotMoveThere,

    #[error("EntityCannotBecameSeeker")]
    EntityCannotBecameSeeker,

    #[error("EntityNotPlayer")]
    EntityNotPlayer,

    #[error("EntityNotHider")]
    EntityNotHider,

    #[error("EntityNotSeeker")]
    EntityNotSeeker,
}

pub struct SeekerHidersSummary {
    pub seeker: Option<(EntityId, SeekerStats)>,
    pub hiders: Vec<(EntityId, HiderStats)>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HiderStats {
    pub covered: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SeekerStats {
    pub remaining_ticks: u32,
    pub remaining_failures: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerRole {
    Hider {
        stats: HiderStats
    },
    Seeker {
        stats: SeekerStats
    }
}

impl Default for PlayerRole {
    fn default() -> Self {
        Self::Hider { stats: HiderStats { covered: true } }
    }
}

#[derive(Debug, Default)]
pub struct PlayerController {
    role: PlayerRole
}

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
        // let colors = [
        //     [255, 0, 0],
        //     [0, 255, 0],
        //     [0, 0, 255],
        //     [255, 0, 255],
        //     [0, 255, 255],
        //     [255, 255, 0],
        // ];
        // let mut rng = rand::rng();
        // let color = *colors.choose(&mut rng).unwrap();
        let channel = rand::random_range(35..150);
        let color = [channel, channel, channel];
        self.create_entity(
            name, 
            intial_position, 
            size,
            color,
            EntityStats {
                movement_speed: PLAYER_MOVEMENT_SPEED
            }, 
            EntityController::Player(PlayerController::default())
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

    pub fn select_entity_as_seeker(&mut self, entity_id: EntityId, remaining_ticks: u32, remaining_failures: usize) -> Result<(), WorldError> {
        assert!(remaining_ticks > 0);
        let entity = match self.get_entity_by_id_mut(entity_id) {
            Some(e) => e,
            None => {
                return Err(WorldError::EntityNotExist);
            }
        };

        match &mut entity.controller {
            EntityController::Npc(_) => {
                Err(WorldError::EntityCannotBecameSeeker)
            },
            EntityController::Player(player_controller) => {
                player_controller.role = PlayerRole::Seeker { stats: SeekerStats { remaining_ticks, remaining_failures } };
                Ok(())
            },
        }
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

    // TODO add some types to distinguish tiled metrics from units overall

    /// range - in units not in tiles
    fn get_tiles_positions(&self, center_point: Vector2F, circle_range: f32, find_free_tiles: bool) -> Vec<Vector2F> {
        assert!(circle_range > 0.0);
        const HALF_TILE: Vector2F = Vector2F {
            x: TILE_SIZE / 2.0,
            y: TILE_SIZE / 2.0
        };
        const ENOUGH_CORNERS_TO_BE_INTERSECTING: usize = 2;

        if circle_range == 0.0 {
            return Vec::default();
        }

        let circle_range_squarde = circle_range.powi(2);

        // range should be higher or equal tocover all possible tiles
        let circle_range_tiled = {
            let tmp = (circle_range / TILE_SIZE).ceil();
            if tmp <= 0.0 {
                0
            } else {
                tmp as i32
            }
        };

        // iterate from to +- range_tiled in x and y and pick those from inside of circle
        let xy_range = -circle_range_tiled..=circle_range_tiled;

        let mut results = vec![];

        for iy in xy_range.clone() {
            for ix in xy_range.clone() {

                let tile_corners= [
                    (ix, iy),
                    (ix + 1, iy),
                    (ix, iy + 1),
                    (ix + 1, iy + 1),
                ];

                let tile_corners_intersecting_count = tile_corners.iter()
                    .filter_map(|(corner_x, corner_y)| {
                        let corner_vec = get_tiled_vec(*corner_x, *corner_y);
                        let center_corner_vec = corner_vec - HALF_TILE;

                        // At edge case will always capture additional 4 adjecent tiles
                        (center_corner_vec.length_squared() <= circle_range_squarde).then_some(())
                    })
                    .count();

                let enough_tile_corners_intersecting = tile_corners_intersecting_count >= ENOUGH_CORNERS_TO_BE_INTERSECTING;

                if enough_tile_corners_intersecting {
                    let tile_position = center_point + get_tiled_vec(ix, iy);

                    // Toglle search
                    if find_free_tiles != self.is_tile_occupied(&tile_position) {
                        results.push(tile_position);
                    }
                }
            }
        }
        results
    }
    
    pub fn get_free_tiles_positions(&self, center_point: Vector2F, circle_range: f32) -> Vec<Vector2F> {
        self.get_tiles_positions(center_point, circle_range, true)
    }

    pub fn get_occupied_tiles_positions(&self, center_point: Vector2F, circle_range: f32) -> Vec<Vector2F> {
        self.get_tiles_positions(center_point, circle_range, false)
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

    pub fn get_seeker_hiders_summary(&self) -> SeekerHidersSummary {
        let mut summary = SeekerHidersSummary {
            seeker: None,
            hiders: vec![]
        };

        for entity in self.entities.iter() {
            let entity_id = entity.id;
            if let EntityController::Player(player_ctrl) = &entity.controller {
                match player_ctrl.role {
                    PlayerRole::Hider { stats } => summary.hiders.push((entity_id, stats)),
                    PlayerRole::Seeker { stats } => summary.seeker = Some((entity_id, stats)),
                }
            }
        }

        summary
    }

    pub fn is_entity_inrange(entity_position_1: Vector2F, entity_position_2: Vector2F) -> bool {
        const ENTITY_MAX_RANGE: f32 = TILE_SIZE * 1.5; // Slightly more than sqrt(2)
        const ENTITY_MAX_RANGE_SQUARED: f32 = ENTITY_MAX_RANGE * ENTITY_MAX_RANGE;
        (entity_position_1 - entity_position_2).length_squared() < ENTITY_MAX_RANGE_SQUARED
    }
    
    pub fn access_seeker_states_mut(&mut self) -> Option<&mut SeekerStats> {
        for entity in self.entities.iter_mut() {
            match &mut entity.controller {
                EntityController::Npc(_) => continue,
                EntityController::Player(player_controller) => {
                    if let PlayerRole::Seeker { stats } = &mut player_controller.role {
                        return Some(stats);
                    }
                },
            }
        }
        None
    }
    
    pub fn access_hiders_states_mut(&mut self) -> Vec<(EntityId, &mut HiderStats)> {
        let mut results = vec![];

        for entity in self.entities.iter_mut() {
            match &mut entity.controller {
                EntityController::Npc(_) => continue,
                EntityController::Player(player_controller) => {
                    if let PlayerRole::Hider { stats } = &mut player_controller.role {
                        results.push((entity.id, stats));
                    }
                },
            }
        }
        
        results
    }

    pub fn tick_seeker_remaining_time(&mut self) {
        if let Some(seeker_stats) = self.access_seeker_states_mut() {
            seeker_stats.remaining_ticks = seeker_stats.remaining_ticks.saturating_sub(1);
        } else {
            log::warn!("Seeker not found!");
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

    pub fn get_player_role(&self) -> Option<&PlayerRole> {
        match &self.controller {
            EntityController::Npc(_) => None,
            EntityController::Player(player_controller) => Some(&player_controller.role),
        }
    }

    pub fn set_hider_covered(&mut self, covered: bool) -> Result<(), WorldError> {
        match &mut self.controller {
            EntityController::Npc(_) => Err(WorldError::EntityNotPlayer),
            EntityController::Player(player_controller) => {
                match &mut player_controller.role {
                    PlayerRole::Hider { stats } => {
                        stats.covered = covered;
                        Ok(())
                    },
                    PlayerRole::Seeker { stats } => {
                        Err(WorldError::EntityNotHider)
                    },
                }
            },
        }
    }

    pub fn punish_seeker(&mut self) -> Result<(), WorldError> {
        match &mut self.controller {
            EntityController::Npc(_) => Err(WorldError::EntityNotPlayer),
            EntityController::Player(player_controller) => {
                match &mut player_controller.role {
                    PlayerRole::Hider { stats } => {
                        Err(WorldError::EntityNotSeeker)
                    },
                    PlayerRole::Seeker { stats } => {
                        stats.remaining_failures = stats.remaining_failures.saturating_sub(1);
                        Ok(())
                    },
                }
            },
        }
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
fn test_world_entity_access_should_align_position() {
    let entity_name = "Bob";
    let entity_position = Vector2F::new(1.0, 2.0);
    let expected_position = Vector2F::new(0.0, 0.0);

    let mut world = World::new();
    let new_entity_id = world.create_entity_npc(entity_name, entity_position, Vector2F::new(1.0, 1.0));

    let entity = world.get_entity_by_id(new_entity_id).unwrap();
    assert_eq!(entity.name, entity_name);
    assert_eq!(entity.position, expected_position);
    assert_eq!(entity.state, EntityState::Idle);
}

#[test]
fn test_world_entity_translate() {
    let entity_initial_position = Vector2F::new(TILE_SIZE, 0.0);
    let translation = Vector2F::new(100.0, 500.0);

    let mut world = World::new();
    let new_entity_id = world.create_entity_npc("Bob", entity_initial_position, Vector2F::new(TILE_SIZE, 0.0));

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

#[test]
fn test_tiled_coords() {
    assert_eq!(0.0, get_tiled_value(0));
    assert_eq!(-TILE_SIZE, get_tiled_value(-1));
    assert_eq!(TILE_SIZE, get_tiled_value(1));
    
    assert_eq!(Vector2F::zero(), get_tiled_vec(0, 0));
    assert_eq!(Vector2F::new(-TILE_SIZE, -TILE_SIZE), get_tiled_vec(-1, -1));
    assert_eq!(Vector2F::new(TILE_SIZE, TILE_SIZE), get_tiled_vec(1, 1));
    assert_eq!(Vector2F::new(2.0 * TILE_SIZE, 0.0), get_tiled_vec(2, 0));
}

#[test]
fn test_world_entity_count_free_positions() {
    let world = World::new();

    let free_tiles_not_enough_range = world.get_free_tiles_positions(Vector2F::zero(), TILE_SIZE / 2.0);
    assert_eq!(free_tiles_not_enough_range.len(), 0);

    let free_tiles_one_tile_edge_case = world.get_free_tiles_positions(Vector2F::zero(), (TILE_SIZE / 2.0) * 2.0_f32.sqrt());
    assert_eq!(free_tiles_one_tile_edge_case.len(), 1 + 4);

    let free_tiles_one_tile_range = world.get_free_tiles_positions(Vector2F::zero(), TILE_SIZE);
    assert_eq!(free_tiles_one_tile_range.len(), 1 + 4);

    let free_tiles_captures_9_tiles = world.get_free_tiles_positions(Vector2F::zero(), 2.0 *TILE_SIZE);
    assert_eq!(free_tiles_captures_9_tiles.len(), 9 + 4);
}

#[test]
fn test_world_entity_free_positions() {
    let entity_name = "Bob";
    let occupied_tile_location = Vector2I::new(1, 1);
    let entity_occupied_posiiton = get_tiled_vec(occupied_tile_location.x, occupied_tile_location.y);

    let mut world = World::new();
    let _new_entity_id = world.create_entity_npc(entity_name, entity_occupied_posiiton, ENTITY_SIZE);

    let free_tiles = world.get_free_tiles_positions(Vector2F::zero(), get_tiled_value(3));

    assert!(!free_tiles.contains(&entity_occupied_posiiton));
}

#[test]
fn test_entities_positions_inrange() {
    let p1 = get_tiled_vec(1, 2);
    let p2 = get_tiled_vec(1, 3);
    assert!(World::is_entity_inrange(p1, p2));
    
    let p1 = get_tiled_vec(1, 2);
    let p2 = get_tiled_vec(2, 3);
    assert!(World::is_entity_inrange(p1, p2));
}

#[test]
fn test_entities_positions_not_inrange() {
    let p1 = get_tiled_vec(1, 2);
    let p2 = get_tiled_vec(1, 4);
    assert!(!World::is_entity_inrange(p1, p2));
    
    let p1 = get_tiled_vec(-1, 0);
    let p2 = get_tiled_vec(1, 0);
    assert!(!World::is_entity_inrange(p1, p2));
}
