use crate::bevy_app::{CurrentVolume, Hovered, InDrag, Selected, Target};
use bevy::{
    ecs::message::MessageReader, input::mouse::MouseWheel, math::bounding::RayCast3d,
    platform::collections::HashMap, prelude::*,
};
use bevy_input::common_conditions::*;
use js_sys::global;
use std::ops::Range;
use wasm_bindgen::JsCast;
use web_sys::DedicatedWorkerGlobalScope;

pub(crate) struct RayPickPlugin;

#[derive(Resource, Default)]
struct CursorPosition {
    position: Vec2,
}

impl Plugin for RayPickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraSettings {
            orthographic_viewport_height: 5.,
            // In orthographic projections, we specify camera scale relative to a default value of 1,
            // in which one unit in world space corresponds to one pixel.
            orthographic_zoom_range: 0.1..3000.0,
            // This value was hand-tuned to ensure that zooming in and out feels smooth but not slow.
            orthographic_zoom_speed: 0.2,
        });
        app.init_resource::<CursorPosition>().add_systems(
            Update,
            (
                (
                    position_system.run_if(on_message::<CursorMoved>),
                    hover_system.run_if(on_message::<CursorMoved>),
                    drag_start_system.run_if(input_just_pressed(MouseButton::Left)),
                    drag_finish_system.run_if(input_just_released(MouseButton::Left)),
                    drag_system.run_if(on_message::<CursorMoved>),
                    select_system.run_if(input_just_pressed(MouseButton::Left)),
                )
                    .chain(),
                zoom_system,
            ),
        );
    }
}

use bevy::input::mouse::AccumulatedMouseScroll;

#[derive(Debug, Resource)]
struct CameraSettings {
    /// The height of the viewport in world units when the orthographic camera's scale is 1
    pub orthographic_viewport_height: f32,
    /// Clamp the orthographic camera's scale to this range
    pub orthographic_zoom_range: Range<f32>,
    /// Multiply mouse wheel inputs by this factor when using the orthographic camera
    pub orthographic_zoom_speed: f32,
}

fn zoom_system(
    camera: Single<&mut Projection, With<Camera>>,
    camera_settings: Res<CameraSettings>,
    mouse_wheel_input: Res<AccumulatedMouseScroll>,
) {
    match *camera.into_inner() {
        Projection::Orthographic(ref mut orthographic) => {
            /*
            info!(
                "Mouse wheel delta: {:?}",
                mouse_wheel_input.delta.y
            );
             */
            // We want scrolling up to zoom in, decreasing the scale, so we negate the delta.
            let delta_zoom = -mouse_wheel_input.delta.y * camera_settings.orthographic_zoom_speed;
            // When changing scales, logarithmic changes are more intuitive.
            // To get this effect, we add 1 to the delta, so that a delta of 0
            // results in no multiplicative effect, positive values result in a multiplicative increase,
            // and negative values result in multiplicative decreases.
            let multiplicative_zoom = 1. + delta_zoom;

            // info!("Orthographic scale: {}", orthographic.scale);
            orthographic.scale = (orthographic.scale * multiplicative_zoom).clamp(
                camera_settings.orthographic_zoom_range.start,
                camera_settings.orthographic_zoom_range.end,
            );
        }
        Projection::Perspective(ref mut perspective) => {
            // We want scrolling up to zoom in, decreasing the scale, so we negate the delta.
            let delta_zoom = -mouse_wheel_input.delta.y * camera_settings.orthographic_zoom_speed;

            // Adjust the field of view, but keep it within our stated range.
            perspective.fov = (perspective.fov + delta_zoom).clamp(
                camera_settings.orthographic_zoom_range.start,
                camera_settings.orthographic_zoom_range.end,
            );
        }
        _ => info!("not orthographic camera, skip zoom"),
    }
}

/*
fn zoom_system(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut query: Query<(Entity, &CurrentVolume, &mut Transform), With<Target>>,
) {
    // TODO: mouse wheel
    for _event in mouse_wheel_events.read() {}
}
*/

fn drag_start_system(
    mut commands: Commands,
    p: Res<CursorPosition>,
    hovered: Query<Entity, (With<Hovered>, Without<InDrag>)>,
) {
    for entity in hovered.iter() {
        commands.entity(entity).insert(InDrag {
            position: p.position,
        });
    }
}

fn drag_finish_system(
    mut commands: Commands,
    in_drag: Query<(Entity, &mut Transform), With<InDrag>>,
) {
    for (entity, _) in in_drag.iter() {
        commands.entity(entity).remove::<InDrag>();
    }
}

fn drag_system(
    mut cursor_moved: MessageReader<CursorMoved>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut in_drag: Query<(Entity, &mut Transform, &mut InDrag), With<InDrag>>,
) {
    if let Some(last) = cursor_moved.read().last() {
        let (camera, global_transform) = cameras.single().unwrap();
        let cur = screen_to_world(last.position, camera, global_transform).unwrap();
        for (entity, mut transform, mut in_drag) in in_drag.iter_mut() {
            let start = screen_to_world(in_drag.position, camera, global_transform).unwrap();
            let offset = cur - start;
            transform.translation += offset;
            in_drag.position = last.position;
        }
    }
}

fn position_system(mut cursor_moved: MessageReader<CursorMoved>, mut p: ResMut<CursorPosition>) {
    if let Some(last) = cursor_moved.read().last() {
        p.position = last.position;
    }
}

fn hover_system(
    mut commands: Commands,
    mut cursor_moved: MessageReader<CursorMoved>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    q: Query<(Entity, &CurrentVolume), With<Target>>,
) {
    let mut hovers: HashMap<Entity, u64> = HashMap::default();
    for ev in cursor_moved.read() {
        let (camera, transform) = cameras.single().unwrap();
        let ray = ray_from_screenspace(ev.position, camera, transform).unwrap();
        let ray_cast = RayCast3d::from_ray(ray, 30.);

        // 计算射线拾取
        for (entity, volume) in q.iter() {
            // 射线求交
            commands.entity(entity).remove::<Hovered>();
            let toi = ray_cast.aabb_intersection_at(volume);
            if toi.is_some() {
                info!("toi: {:?}, entity: {:?}", toi, entity);
                commands.entity(entity).insert(Hovered {});
                hovers.insert(entity, entity.to_bits());
            }
        }
    }

    // 通知 js pick 结果
    if let Ok(global) = global().dyn_into::<DedicatedWorkerGlobalScope>() {
        let picks: Vec<u64> = hovers.values().copied().collect();
        info!("[worker] -> hover: {:?}", &picks);
        let msg = super::web_ffi::Message {
            ty: "pick".to_string(),
            list: Some(picks),
            ..default()
        };

        let val = serde_wasm_bindgen::to_value(&msg).unwrap();
        global.post_message(&val).unwrap();
    }
}

fn select_system(
    mut commands: Commands,
    old: Query<Entity, With<Selected>>,
    hovered: Query<Entity, With<Hovered>>,
) {
    info!("[worker] left button pressed, re-select ...",);
    for entity in old.iter() {
        commands.entity(entity).remove::<Selected>();
    }
    for entity in hovered.iter() {
        commands.entity(entity).insert(Selected {});
    }
}

/// 构造一条相机射线
fn ray_from_screenspace(
    cursor_pos_screen: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Ray3d> {
    let mut viewport_pos = cursor_pos_screen;
    if let Some(viewport) = &camera.viewport {
        viewport_pos -= viewport.physical_position.as_vec2();
    }
    camera
        .viewport_to_world(camera_transform, viewport_pos)
        .ok()
}

fn screen_to_world(
    pixel_pos: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Vec3> {
    let ray = ray_from_screenspace(pixel_pos, camera, camera_transform);
    if let Some(ray) = ray {
        // 射线与对像所有平面的交点
        let d = ray.intersect_plane(Vec3::new(0., 0., 2.), InfinitePlane3d::new(Vec3::Z));
        if let Some(d) = d {
            return Some(ray.origin + ray.direction * d);
        }
    }
    None
}
