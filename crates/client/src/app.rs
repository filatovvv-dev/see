//! Bevy-плагин клиента: ресурс симуляции, камера, ввод.

use bevy::prelude::*;

use crate::render::{spawn_agent_sprites, update_agent_sprites};
use crate::sim_bridge::SimBridge;
use crate::ui::{setup_ui, update_ui};

/// Шаг изменения скорости по клавишам Up/Down.
const SPEED_STEP: f32 = 0.5;

/// Плагин SEE.
pub struct SeePlugin;

impl Plugin for SeePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimResource(SimBridge::new(42, 5)))
            .add_systems(Startup, (setup_camera, spawn_agent_sprites, setup_ui))
            .add_systems(
                Update,
                (keyboard_input, sim_update, update_agent_sprites, update_ui).chain(),
            );
    }
}

/// Ресурс: `SimBridge` в ECS.
#[derive(Resource)]
pub struct SimResource(pub SimBridge);

/// Создаёт 2D-камеру.
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Обновляет `SimBridge` каждый кадр.
fn sim_update(time: Res<Time>, mut sim: ResMut<SimResource>) {
    sim.0.update(time.delta_secs());
}

/// Клавиши: Space — пауза, Up/Down — скорость, Esc — выход.
fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut sim: ResMut<SimResource>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Space) {
        sim.0.toggle_pause();
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        let speed = sim.0.speed + SPEED_STEP;
        sim.0.set_speed(speed);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        let speed = sim.0.speed - SPEED_STEP;
        sim.0.set_speed(speed);
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
