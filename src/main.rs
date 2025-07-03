use bevy::prelude::*;

mod game;

// Enum to manage the high-level state of the application
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    MainMenu,
    InGame,
    GameOver, // Added GameOver state
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Chroma Maze".into(),
                resolution: (1000.0, 800.0).into(),
                present_mode: bevy::window::PresentMode::AutoNoVsync, // Try to disable vsync for headless
                ..default()
            }),
            ..default()
        }))
        // The correct method for Bevy 0.14 is init_state
        .init_state::<GameState>()
        .add_systems(Startup, (
            setup_camera,
            // Simple state transition for starting the game
            |mut next_state: ResMut<NextState<GameState>>| {
                next_state.set(GameState::InGame);
                println!("Switched to InGame state");
            }
        ))
        .add_systems(OnEnter(GameState::GameOver), || {
            println!("Entered GameOver state!");
        })
        .add_plugins(game::GamePlugin)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}