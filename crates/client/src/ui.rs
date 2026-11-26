//! Минимальный UI: текстовый блок с состоянием симуляции.

use bevy::prelude::*;

use crate::app::SimResource;

/// Сколько последних событий показывать.
const SHOWN_EVENTS: usize = 3;

/// Путь к шрифту с кириллицей относительно каталога `assets` крейта `client`
/// (Bevy ищет ресурсы в `$CARGO_MANIFEST_DIR/assets`, то есть в
/// `crates/client/assets`).
pub const FONT_PATH: &str = "fonts/NotoSans-Regular.ttf";

/// Максимум символов сцены в UI.
const SCENE_MAX_CHARS: usize = 200;

/// Маркер: текстовый блок с информацией.
#[derive(Component)]
pub struct InfoText;

/// Создаёт текстовый блок в левом верхнем углу.
pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);
    commands.spawn((
        Text::new("SEE"),
        TextFont {
            font,
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
    if let Some(scene) = &bridge.last_scene {
        let shown: String = scene.chars().take(SCENE_MAX_CHARS).collect();
        let ellipsis = if scene.chars().count() > SCENE_MAX_CHARS {
            "…"
        } else {
            ""
        };
        text.push_str(&format!("\n\nСцена:\n{shown}{ellipsis}"));
    }
    for mut ui_text in &mut query {
        ui_text.0.clone_from(&text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_file_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(FONT_PATH);
        assert!(path.exists(), "font not found: {}", path.display());
    }

    #[test]
    fn font_path_is_relative_to_assets() {
        assert!(!FONT_PATH.starts_with("assets"));
        assert!(!FONT_PATH.starts_with('/'));
    }
}
