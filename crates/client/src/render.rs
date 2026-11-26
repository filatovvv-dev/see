//! Отрисовка агентов: цветные круги на 2D-сцене.

use bevy::prelude::*;
use sim_core::entities::AgentId;

use crate::app::SimResource;
use crate::sim_bridge::agent_color;

/// Радиус раскладки агентов, px.
const LAYOUT_RADIUS: f32 = 200.0;

/// Радиус круга агента, px.
const SPRITE_RADIUS: f32 = 10.0;

/// Маркер: спрайт агента.
#[derive(Component)]
pub struct AgentSprite(pub AgentId);

/// Ресурс: последнее поколение мира, для которого созданы спрайты.
#[derive(Resource, Default)]
pub struct SpritesGeneration(pub u64);

/// Пересоздаёт спрайты, если поколение мира изменилось (Update).
///
/// Начальное поколение моста — 1, а `SpritesGeneration::default()` — 0,
/// поэтому первый вызов создаёт спрайты, и отдельная Startup-система не
/// нужна.
pub fn sync_agent_sprites(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut sprites_generation: ResMut<SpritesGeneration>,
    existing: Query<Entity, With<AgentSprite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if sprites_generation.0 == sim.0.generation {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let mesh = meshes.add(Circle::new(SPRITE_RADIUS));
    for (id, pos) in sim.0.agent_layout(LAYOUT_RADIUS) {
        let [r, g, b, a] = agent_color(id);
        commands.spawn((
            AgentSprite(id),
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(r, g, b, a)))),
            Transform::from_xyz(pos.x, pos.y, 0.0),
        ));
    }
    sprites_generation.0 = sim.0.generation;
}

/// Обновляет позиции спрайтов по текущему тику (Update).
pub fn update_agent_sprites(
    sim: Res<SimResource>,
    mut query: Query<(&AgentSprite, &mut Transform)>,
) {
    let positions = sim.0.agent_positions(LAYOUT_RADIUS);
    for (sprite, mut transform) in &mut query {
        if let Some(pos) = positions.get(&sprite.0) {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}
