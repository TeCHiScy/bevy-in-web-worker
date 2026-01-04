use crate::WorkerApp;
use crate::ray_pick::RayPickPlugin;
use crate::{OffscreenCanvas, OffscreenCanvasPlugin};
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::{basic::SILVER, css::BLANCHED_ALMOND, tailwind::BLUE_400},
    math::bounding::{Aabb3d, Bounded3d},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rand::Rng;
use std::f32::consts::PI;
use std::ops::Deref;

pub(crate) fn init_app(canvas: web_sys::OffscreenCanvas, scale_factor: f32) -> WorkerApp {
    let canvas = OffscreenCanvas::new(canvas, scale_factor);
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(bevy::window::WindowPlugin {
                primary_window: Some(bevy::window::Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
        RayPickPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, (rotate, update_aabbes))
    .add_systems(PostUpdate, (render_hovered_shapes, render_selected_shapes))
    .add_plugins(OffscreenCanvasPlugin)
    .insert_resource(canvas);

    WorkerApp::new(app, scale_factor)
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component, Clone)]
enum Shape {
    Box(Cuboid),
}

#[derive(Component)]
pub(crate) struct Hovered {}

#[derive(Component)]
pub(crate) struct Target {}

#[derive(Component)]
pub(crate) struct Selected {}

#[derive(Component)]
pub(crate) struct InDrag {
    pub position: Vec2,
}

const X_EXTENT: f32 = 13.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let meshe_handles = [
        meshes.add(Cuboid::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cuboid::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
    ];
    // 包围盒形状
    let shapes = [
        Shape::Box(Cuboid::from_size(Vec3::splat(1.1))),
        Shape::Box(Cuboid::from_size(Vec3::new(1., 2., 1.))),
        Shape::Box(Cuboid::from_size(Vec3::new(1.75, 0.52, 1.75))),
        Shape::Box(Cuboid::default()),
        Shape::Box(Cuboid::from_size(Vec3::new(1., 2., 1.))),
        Shape::Box(Cuboid::default()),
        Shape::Box(Cuboid::from_size(Vec3::splat(1.1))),
        Shape::Box(Cuboid::default()),
    ];

    let num_shapes = meshe_handles.len();
    let mut rng = rand::rng();

    for i in 0..num_shapes {
        for y in 0..5 {
            for z in 0..1 {
                let index = rng.random_range(0..num_shapes);
                let mesh = meshe_handles[index].to_owned();
                let shape = shapes[index].to_owned();
                let transform = Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    (3.0 - y as f32) * 3. - 2.0,
                    2. + 4.5 * z as f32,
                );

                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(debug_material.clone()),
                    transform.with_rotation(Quat::from_rotation_x(-PI / 4.)),
                    shape,
                    Target {},
                ));
            }
        }
    }

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 20_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 4.0, 16.0),
    ));

    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
        Transform::IDENTITY.with_rotation(Quat::from_rotation_x(PI / 2.)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, -9., 18.0).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
    ));
}

fn rotate(mut q: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut q {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

fn render_hovered_shapes(
    mut gizmos: Gizmos,
    q: Query<(&Shape, &Transform), (With<Hovered>, Without<Selected>)>,
) {
    for (shape, transform) in q.iter() {
        let translation = transform.translation.xyz();
        match shape {
            Shape::Box(cuboid) => {
                gizmos.primitive_3d(
                    cuboid,
                    Isometry3d::new(translation, transform.rotation),
                    BLANCHED_ALMOND,
                );
            }
        }
    }
}

fn render_selected_shapes(mut gizmos: Gizmos, q: Query<(&Shape, &Transform), With<Selected>>) {
    for (shape, transform) in q.iter() {
        let translation = transform.translation.xyz();
        match shape {
            Shape::Box(cuboid) => {
                gizmos.primitive_3d(
                    cuboid,
                    Isometry3d::new(translation, transform.rotation),
                    BLUE_400,
                );
            }
        }
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// entity 的 aabb
#[derive(Component, Debug)]
pub struct CurrentVolume(Aabb3d);

impl Deref for CurrentVolume {
    type Target = Aabb3d;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 更新 aabb
fn update_aabbes(
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
    query: Query<(Entity, &Shape, &Transform), Or<(Changed<Shape>, Changed<Transform>)>>,
) {
    for (_, config, _) in config_store.iter_mut() {
        config.line.width = 3.;
    }

    for (entity, shape, transform) in query.iter() {
        let translation = transform.translation;
        let rotation = transform.rotation;

        let aabb = match shape {
            Shape::Box(b) => b.aabb_3d(Isometry3d::new(translation, rotation)),
        };
        commands.entity(entity).insert(CurrentVolume(aabb));
    }
}
