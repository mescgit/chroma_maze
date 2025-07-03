use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use petgraph::graph::{NodeIndex, UnGraph};
#[allow(unused_imports)] // We will use astar later
use petgraph::algo::astar;
use rand::seq::SliceRandom;
use rand::thread_rng;
// Import the new color palettes
use bevy::color::palettes::css;

use crate::GameState;

// Sizing constants for our grid
const MAP_WIDTH: usize = 40;
const MAP_HEIGHT: usize = 30;
const TILE_SIZE: f32 = 20.0;

// --- New Resource ---
#[derive(Resource)]
struct EnemySpawnTimer(Timer);

// --- Plugin ---
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(ShapePlugin)
            .insert_resource(EnemySpawnTimer(Timer::from_seconds(2.0, TimerMode::Repeating)))
            .add_systems(OnEnter(GameState::InGame), setup_game)
            .add_systems(Update, (
                tower_targeting_system,
                tower_shooting_system, // Added tower_shooting_system
                enemy_death_system,    // Added enemy_death_system
                spawn_enemies_system,
                move_enemies_system,
                check_nexus_health_system,
            ).run_if(in_state(GameState::InGame)));
    }
}

fn enemy_death_system(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Enemy)>, // Query for enemies and their health
) {
    for (entity, enemy) in enemy_query.iter() {
        if enemy.health <= 0.0 {
            commands.entity(entity).despawn();
            println!("Enemy {:?} has been slain!", entity);
        }
    }
}

fn tower_shooting_system(
    mut tower_query: Query<(Entity, &mut Tower)>, // Added Entity to identify the tower for logging
    mut enemy_query: Query<&mut Enemy>, // Query to access and modify enemy health
    time: Res<Time>,
) {
    for (tower_entity, mut tower) in tower_query.iter_mut() {
        if let Some(target_entity) = tower.target {
            tower.fire_timer.tick(time.delta());
            if tower.fire_timer.just_finished() {
                if let Ok(mut enemy) = enemy_query.get_mut(target_entity) {
                    enemy.health -= 1.0; // Deal 1 damage for now
                    println!(
                        "Tower {:?} fired at {:?}. Enemy health: {}",
                        tower_entity, target_entity, enemy.health
                    );
                } else {
                    // This can happen if the enemy is despawned between targeting and shooting
                    println!(
                        "Tower {:?} fired at {:?}, but target was not found (likely despawned).",
                        tower_entity, target_entity
                    );
                    // Optionally, clear the target if it's confirmed invalid,
                    // though tower_targeting_system should also handle this.
                    // tower.target = None;
                }
                // Timer resets automatically due to TimerMode::Repeating
            }
        }
    }
}

// --- New System ---
fn check_nexus_health_system(
    nexus_query: Query<&Nexus>,
    mut next_state: ResMut<NextState<GameState>>, // To change the game state
) {
    if let Ok(nexus) = nexus_query.get_single() {
        if nexus.health <= 0.0 {
            println!("Game Over! Nexus destroyed.");
            next_state.set(GameState::GameOver);
        }
    }
}

// --- Components & Resources ---
#[derive(Component)]
struct MazeTile;

#[derive(Component)]
struct Nexus {
    #[allow(dead_code)]
    health: f32,
}

#[derive(Component)]
#[allow(dead_code)]
struct Enemy {
    speed: f32,
    health: f32,
    path: Vec<(usize, usize)>,
}

#[derive(Component)]
struct Tower {
    fire_rate: f32,
    range: f32,
    target: Option<Entity>,
    fire_timer: Timer, // Added fire_timer field
}

#[derive(Resource)]
pub struct Maze {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileType>,
    #[allow(dead_code)]
    pub graph: UnGraph<(), ()>,
    #[allow(dead_code)]
    pub entrance: (usize, usize),
    pub nexus_pos: (usize, usize),
    // Added the node_map field here
    pub node_map: Vec<NodeIndex>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TileType {
    Wall,
    Floor,
}

// --- Systems ---
fn setup_game(mut commands: Commands) {
    let maze = generate_maze(MAP_WIDTH, MAP_HEIGHT);

    for y in 0..maze.height {
        for x in 0..maze.width {
            let tile_type = maze.tiles[y * maze.width + x];
            let position = Vec3::new(
                (x as f32 - maze.width as f32 / 2.0) * TILE_SIZE,
                (y as f32 - maze.height as f32 / 2.0) * TILE_SIZE,
                0.0,
            );
            
            let color = match tile_type {
                TileType::Wall => css::DARK_GRAY.into(),
                TileType::Floor => css::BLACK.into(),
            };

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(position),
                    ..default()
                },
                MazeTile,
            ));
        }
    }

    let nexus_pos_world = Vec3::new(
        (maze.nexus_pos.0 as f32 - maze.width as f32 / 2.0) * TILE_SIZE,
        (maze.nexus_pos.1 as f32 - maze.height as f32 / 2.0) * TILE_SIZE,
        1.0,
    );

    commands.spawn((
        ShapeBundle {
            path: GeometryBuilder::build_as(&shapes::RegularPolygon {
                sides: 6,
                feature: shapes::RegularPolygonFeature::Radius(TILE_SIZE * 0.8),
                ..shapes::RegularPolygon::default()
            }),
            spatial: SpatialBundle {
                transform: Transform::from_translation(nexus_pos_world),
                ..default()
            },
            mesh: default(),
            material: default(),
        },
        Fill::color(css::AQUA),
        Stroke::new(css::WHITE, 2.0),
        Nexus { health: 100.0 },
    ));
    
    commands.insert_resource(maze);

    // Spawn a test tower
    let tower_position_world = Vec3::new(0.0, 0.0, 1.0); // Example position
    commands.spawn((
        Tower {
            fire_rate: 1.0,
            range: 150.0, // Adjusted for TILE_SIZE units
            target: None,
            fire_timer: Timer::from_seconds(1.0, TimerMode::Repeating), // Initialize fire_timer
        },
        ShapeBundle { // Using ShapeBundle for a simple visual representation
            path: GeometryBuilder::build_as(&shapes::RegularPolygon {
                sides: 3, // Triangle
                feature: shapes::RegularPolygonFeature::Radius(TILE_SIZE * 0.6),
                ..shapes::RegularPolygon::default()
            }),
            spatial: SpatialBundle {
                transform: Transform::from_translation(tower_position_world),
                ..default()
            },
            mesh: default(),
            material: default(),
        },
        Fill::color(css::YELLOW), // Removed .into()
        Stroke::new(css::BLACK, 1.0),
    ));
    println!("Spawned a test tower at (0,0)");
    
    println!("Game setup complete. Maze generated.");
}

fn tower_targeting_system(
    mut tower_query: Query<(Entity, &mut Tower, &GlobalTransform)>,
    enemy_query: Query<(Entity, &GlobalTransform), With<Enemy>>, // Query enemies with their transforms
) {
    for (_tower_entity, mut tower, tower_transform) in tower_query.iter_mut() {
        let tower_position = tower_transform.translation();

        // Check current target
        if let Some(target_entity) = tower.target {
            if let Ok((_enemy_entity, enemy_transform)) = enemy_query.get(target_entity) {
                let target_position = enemy_transform.translation();
                if tower_position.distance(target_position) > tower.range {
                    // Target out of range
                    tower.target = None;
                    println!("Tower {:?} lost target {:?} (out of range)", _tower_entity, target_entity);
                }
                // Else, target is still valid and in range, do nothing
            } else {
                // Target no longer exists (e.g., despawned)
                tower.target = None;
                println!("Tower {:?} lost target {:?} (despawned)", _tower_entity, target_entity);
            }
        }

        // If no target, find a new one
        if tower.target.is_none() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                let enemy_position = enemy_transform.translation();
                if tower_position.distance(enemy_position) <= tower.range {
                    tower.target = Some(enemy_entity);
                    println!("Tower {:?} acquired new target: {:?}", _tower_entity, enemy_entity);
                    break; // Target the first enemy in range
                }
            }
        }
    }
}

fn spawn_enemies_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    maze: Res<Maze>,
) {
    if spawn_timer.0.tick(time.delta()).just_finished() {
        if let Some(path) = find_path(&maze, maze.entrance, maze.nexus_pos) {
            let start_pos_world = Vec3::new(
                (maze.entrance.0 as f32 - maze.width as f32 / 2.0) * TILE_SIZE,
                (maze.entrance.1 as f32 - maze.height as f32 / 2.0) * TILE_SIZE,
                1.0,
            );

            commands.spawn((
                Enemy {
                    speed: 50.0,
                    health: 10.0,
                    path,
                },
                SpriteBundle {
                    sprite: Sprite {
                        color: css::RED.into(),
                        custom_size: Some(Vec2::new(TILE_SIZE * 0.7, TILE_SIZE * 0.7)),
                        ..default()
                    },
                    transform: Transform::from_translation(start_pos_world),
                    ..default()
                }
            ));
            println!("Spawned an enemy!");
        }
    }
}

fn move_enemies_system(
    mut commands: Commands,
    mut enemy_query: Query<(Entity, &mut Transform, &mut Enemy)>, // Added Entity
    mut nexus_query: Query<&mut Nexus>, // Query for Nexus
    time: Res<Time>,
    maze: Res<Maze>,
) {
    for (enemy_entity, mut transform, mut enemy) in enemy_query.iter_mut() {
        if let Some(&next_waypoint_coords) = enemy.path.first() {
            let next_waypoint_world = Vec3::new(
                (next_waypoint_coords.0 as f32 - maze.width as f32 / 2.0) * TILE_SIZE,
                (next_waypoint_coords.1 as f32 - maze.height as f32 / 2.0) * TILE_SIZE,
                transform.translation.z,
            );

            let direction = (next_waypoint_world - transform.translation).normalize_or_zero();
            transform.translation += direction * enemy.speed * time.delta_seconds();

            if transform.translation.distance(next_waypoint_world) < 1.0 {
                enemy.path.remove(0);
            }
        } else {
            // Enemy reached the nexus
            commands.entity(enemy_entity).despawn();
            if let Ok(mut nexus) = nexus_query.get_single_mut() {
                nexus.health -= 10.0; // Decrease nexus health by 10 (arbitrary value for now)
                println!("Nexus health: {}", nexus.health);
            }
        }
    }
}


// --- Maze Generation Logic ---
fn generate_maze(width: usize, height: usize) -> Maze {
    let mut tiles = vec![TileType::Wall; width * height];
    let mut visited = vec![false; width * height];
    let mut stack = Vec::new();
    let mut rng = thread_rng();

    let start_x = 1;
    let start_y = 1;
    let start_idx = start_y * width + start_x;

    visited[start_idx] = true;
    tiles[start_idx] = TileType::Floor;
    stack.push((start_x, start_y));

    while let Some((cx, cy)) = stack.pop() {
        let mut neighbors = Vec::new();
        for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
            let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
            if nx > 0 && nx < width as i32 && ny > 0 && ny < height as i32 {
                let (nx, ny) = (nx as usize, ny as usize);
                if !visited[ny * width + nx] {
                    neighbors.push((nx, ny));
                }
            }
        }

        if !neighbors.is_empty() {
            stack.push((cx, cy));
            let (nx, ny) = *neighbors.choose(&mut rng).unwrap();
            let n_idx = ny * width + nx;

            let wall_x = (cx + nx) / 2;
            let wall_y = (cy + ny) / 2;
            tiles[wall_y * width + wall_x] = TileType::Floor;
            tiles[n_idx] = TileType::Floor;
            
            visited[n_idx] = true;
            stack.push((nx, ny));
        }
    }
    
    let mut graph = UnGraph::new_undirected();
    let mut node_map = vec![NodeIndex::end(); width * height];

    for y in 0..height {
        for x in 0..width {
            if tiles[y * width + x] == TileType::Floor {
                let node = graph.add_node(());
                node_map[y * width + x] = node;
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if tiles[y * width + x] == TileType::Floor {
                let current_node = node_map[y * width + x];
                if x + 1 < width && tiles[y * width + (x + 1)] == TileType::Floor {
                    let right_node = node_map[y * width + (x + 1)];
                    graph.add_edge(current_node, right_node, ());
                }
                if y + 1 < height && tiles[(y + 1) * width + x] == TileType::Floor {
                    let down_node = node_map[(y + 1) * width + x];
                    graph.add_edge(current_node, down_node, ());
                }
            }
        }
    }

    let entrance = (1, 1);
    let nexus_pos = (width / 2, height / 2);
    if tiles[nexus_pos.1 * width + nexus_pos.0] == TileType::Wall {
       tiles[nexus_pos.1 * width + nexus_pos.0] = TileType::Floor;
    }

    // Now we return the node_map as part of the Maze struct
    Maze { width, height, tiles, graph, entrance, nexus_pos, node_map }
}

// A helper function to find a path through the maze.
fn find_path(
    maze: &Maze,
    start_pos: (usize, usize),
    end_pos: (usize, usize),
) -> Option<Vec<(usize, usize)>> {
    let start_node = maze.node_map[start_pos.1 * maze.width + start_pos.0];
    let end_node = maze.node_map[end_pos.1 * maze.width + end_pos.0];

    let result = astar(
        &maze.graph,
        start_node,
        |finish| finish == end_node,
        |_| 1,
        |_| 0,
    );

    if let Some((_cost, path_indices)) = result {
        let path_coords = path_indices.into_iter().map(|node_idx| {
            let flat_index = maze.node_map.iter().position(|&n| n == node_idx).unwrap();
            (flat_index % maze.width, flat_index / maze.width)
        }).collect();
        Some(path_coords)
    } else {
        None
    }
}