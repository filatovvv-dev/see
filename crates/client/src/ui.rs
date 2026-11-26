//! Минимальный UI: текстовый блок с состоянием симуляции.

use bevy::prelude::*;

use crate::app::SimResource;

/// Сколько последних событий показывать.
const SHOWN_EVENTS: usize = 3;

/// Маркер: текстовый блок с информацией.
#[derive(Component)]
pub struct InfoText;

/// Создаёт текстовый блок в левом верхнем углу.
pub fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("SEE"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        InfoText,
    ));
}

/// Обновляет текст: тик, агенты, скорость, пауза, последние события.
pub fn update_ui(sim: Res<SimResource>, mut query: Query<&mut Text, With<InfoText>>) {
    let bridge = &sim.0;
    let events = bridge.recent_events(SHOWN_EVENTS);
    let skip = events.len().saturating_sub(SHOWN_EVENTS);
    let mut text = format!(
        "Тик: {}\nАгенты: {}\nСкорость: {:.1}\nПауза: {}\nСобытия:",
        bridge.world.time.tick,
        bridge.world.agents.len(),
        bridge.speed,
        if bridge.paused { "да" } else { "нет" },
    );
    for event in events.into_iter().skip(skip) {
        text.push_str(&format!("\n  {event:?}"));
    }
    for mut ui_text in &mut query {
        ui_text.0.clone_from(&text);
    }
}
