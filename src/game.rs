use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use petgraph::graph::{NodeIndex, UnGraph};
#[allow(unused_imports)] // We will use astar later
use petgraph::algo::astar;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::GameState;

// Sizing constants for our grid
const MAP_WIDTH: usize = 40;
const MAP_HEIGHT: usize = 30;
const TILE_SIZE: f32 = 20.0;

// --- Plugin ---
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(ShapePlugin)
            .add_systems(OnEnter(GameState::InGame), setup_game)
            .add_systems(Update, (
                tower_targeting_system,
            ).run_if(in_state(GameState::InGame)));
    }
}

// --- Components & Resources ---
#[derive(Component)]
struct MazeTile;

#[derive(Component)]
struct Nexus {
    health: f32,
}

#[derive(Component)]
struct Enemy {
    speed: f32,
    health: f32,
    path: Vec<NodeIndex>,
}

#[derive(Component)]
struct Tower {
    fire_rate: f32,
    range: f32,
    target: Option<Entity>,
}

#[derive(Resource)]
pub struct Maze {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileType>,
    pub graph: UnGraph<(), ()>,
    pub entrance: (usize, usize),
    pub nexus_pos: (usize, usize),
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
                TileType::Wall => Color::DARK_GRAY,
                TileType::Floor => Color::BLACK,
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
            transform: Transform::from_translation(nexus_pos_world),
            ..default()
        },
        Fill::color(Color::CYAN),
        Stroke::new(Color::WHITE, 2.0),
        Nexus { health: 100.0 },
    ));
    
    commands.insert_resource(maze);
    
    println!("Game setup complete. Maze generated.");
}

fn tower_targeting_system() { /* ... */ }

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
    
    // The name `new_unweighted` is correct for petgraph 0.6.4
    let mut graph = UnGraph::new_unweighted();
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

    Maze { width, height, tiles, graph, entrance, nexus_pos }
}

// Prefixing `maze` with an underscore to silence the unused variable warning
fn find_path_example(_maze: &Maze) { /* ... */ }