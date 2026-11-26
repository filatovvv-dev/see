//! Отрисовка агентов: цветные круги на 2D-сцене.

use bevy::prelude::*;
use sim_core::entities::{AgentId, ShipId, SystemId};

use crate::app::SimResource;
use crate::sim_bridge::{agent_color, planet_color, system_color};

/// Радиус раскладки агентов, px.
const LAYOUT_RADIUS: f32 = 200.0;

/// Радиус раскладки систем, px.
const SYSTEM_LAYOUT_RADIUS: f32 = 350.0;

/// Радиус круга системы, px.
const SYSTEM_SPRITE_RADIUS: f32 = 30.0;

/// Радиус орбиты планет вокруг системы, px.
const PLANET_ORBIT_RADIUS: f32 = 50.0;

/// Радиус круга планеты, px.
const PLANET_SPRITE_RADIUS: f32 = 6.0;

/// Радиус круга корабля, px.
const SHIP_SPRITE_RADIUS: f32 = 4.0;

/// Радиус круга агента, px.
const SPRITE_RADIUS: f32 = 10.0;

/// Маркер: спрайт агента.
#[derive(Component)]
pub struct AgentSprite(pub AgentId);

/// Маркер: спрайт системы.
#[derive(Component)]
pub struct SystemSprite(pub SystemId);

/// Маркер: спрайт планеты (декорация).
#[derive(Component)]
pub struct PlanetSprite;

/// Маркер: спрайт корабля.
#[derive(Component)]
pub struct ShipSprite(pub ShipId);

/// Фильтр: любой спрайт системы, планеты или корабля.
type WorldSpriteFilter = Or<(With<SystemSprite>, With<PlanetSprite>, With<ShipSprite>)>;

/// Ресурс: последнее поколение мира, для которого созданы системы, планеты
/// и корабли.
#[derive(Resource, Default)]
pub struct WorldSpritesGeneration(pub u64);

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

/// Пересоздаёт системы, планеты и корабли, если поколение мира изменилось
/// (Update). Начальное значение ресурса — 0, а у моста — 1, поэтому первый
/// вызов создаёт спрайты.
pub fn sync_world_sprites(
    mut commands: Commands,
    sim: Res<SimResource>,
    mut world_generation: ResMut<WorldSpritesGeneration>,
    existing: Query<Entity, WorldSpriteFilter>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if world_generation.0 == sim.0.generation {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let system_mesh = meshes.add(Circle::new(SYSTEM_SPRITE_RADIUS));
    let planet_mesh = meshes.add(Circle::new(PLANET_SPRITE_RADIUS));
    let ship_mesh = meshes.add(Circle::new(SHIP_SPRITE_RADIUS));

    for (id, pos) in sim.0.system_layout(SYSTEM_LAYOUT_RADIUS) {
        let [r, g, b, a] = system_color(id);
        commands.spawn((
            SystemSprite(id),
            Mesh2d(system_mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(r, g, b, a)))),
            Transform::from_xyz(pos.x, pos.y, -2.0),
        ));
        for (j, planet) in sim
            .0
            .planet_layout(id, PLANET_ORBIT_RADIUS)
            .iter()
            .enumerate()
        {
            let [r, g, b, a] = planet_color(j);
            commands.spawn((
                PlanetSprite,
                Mesh2d(planet_mesh.clone()),
                MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(r, g, b, a)))),
                Transform::from_xyz(pos.x + planet.x, pos.y + planet.y, -1.0),
            ));
        }
    }

    let ship_material = materials.add(ColorMaterial::from(Color::srgb(0.85, 0.85, 0.85)));
    for (id, pos) in sim.0.ship_positions(SYSTEM_LAYOUT_RADIUS) {
        commands.spawn((
            ShipSprite(id),
            Mesh2d(ship_mesh.clone()),
            MeshMaterial2d(ship_material.clone()),
            Transform::from_xyz(pos.x, pos.y, 1.0),
        ));
    }
    world_generation.0 = sim.0.generation;
}

/// Обновляет позиции кораблей (Update).
pub fn update_ship_sprites(sim: Res<SimResource>, mut query: Query<(&ShipSprite, &mut Transform)>) {
    let positions = sim.0.ship_positions(SYSTEM_LAYOUT_RADIUS);
    for (sprite, mut transform) in &mut query {
        if let Some(pos) = positions.get(&sprite.0) {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}
