use avian2d::prelude::*;
use bevy::{prelude::*, window::WindowMode};
use rand::prelude::*;
use std::mem::discriminant;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Numbered(u32);

#[derive(Event)]
struct MovementAction(i32);

#[derive(Event)]
struct BallHitEvent {
    ball_id: Entity,
    ball_number: u32,
}

#[derive(Resource)]
struct BallSpawnTimer(Timer);

const STARTING_NUMBER: u32 = 5;

#[derive(Copy, Clone)]
enum Bound {
    UpperBound,
    LowerBound,
    LeftBound,
    RightBound,
}

impl Bound {
    const VARIANTS: [Bound; 4] = [
        Bound::UpperBound,
        Bound::LowerBound,
        Bound::LeftBound,
        Bound::RightBound,
    ];

    fn value(&self) -> f32 {
        match self {
            Bound::UpperBound => -500.,
            Bound::LowerBound => 500.,
            Bound::LeftBound => -940.,
            Bound::RightBound => 940.,
        }
    }

    fn other_random(&self) -> Self {
        let mut rng = rand::rng();
        let other_variants: Vec<Bound> = Self::VARIANTS
            .into_iter()
            .filter(|v| discriminant(v) != discriminant(self))
            .collect();

        *other_variants.choose(&mut rng).unwrap()
    }

    fn random() -> Self {
        let mut rng = rand::rng();

        *Self::VARIANTS.choose(&mut rng).unwrap()
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            PhysicsPlugins::default(),
        ))
        .insert_resource(Gravity(Vec2::NEG_Y * 1000.))
        .insert_resource(BallSpawnTimer(Timer::from_seconds(
            0.5,
            TimerMode::Repeating,
        )))
        .add_event::<MovementAction>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                keyboard_input,
                change_gravity,
                movement,
                spawn_ball,
                despawn_out_of_bounds_balls,
                detect_hits,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((RigidBody::Dynamic, Collider::circle(0.5)));

    let walls = [
        ((1880., 20.), (0., Bound::UpperBound.value())),
        ((1880., 20.), (0., Bound::LowerBound.value())),
        ((20., 1020.), (Bound::LeftBound.value(), 0.)),
        ((20., 1020.), (Bound::RightBound.value(), 0.)),
    ];

    for (size, transform) in walls {
        commands.spawn((
            Sprite {
                color: Color::srgb(0.0, 0.4, 0.7),
                custom_size: Some(Vec2::new(size.0, size.1)),
                ..default()
            },
            Transform::from_xyz(transform.0, transform.1, 100.),
            RigidBody::Static,
            Collider::rectangle(size.0, size.1),
            Restitution::PERFECTLY_INELASTIC,
        ));
    }

    commands
        .spawn((
            Player,
            CollidingEntities::default(),
            Numbered(STARTING_NUMBER),
            Mesh2d(meshes.add(Rectangle::new(30., 30.))),
            MeshMaterial2d(materials.add(Color::srgb(0., 0., 1.))),
            Transform::from_xyz(200., 0., 0.),
            RigidBody::Dynamic,
            Collider::rectangle(30., 30.),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text2d::new(STARTING_NUMBER.to_string()),
                TextFont {
                    font_size: 20.,
                    ..default()
                },
            ));
        });
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut movement_event_writer: EventWriter<MovementAction>,
) {
    if keys.pressed(KeyCode::KeyD) {
        movement_event_writer.send(MovementAction(1));
    }
    if keys.pressed(KeyCode::KeyA) {
        movement_event_writer.send(MovementAction(-1));
    }
}

fn movement(
    mut movement_event_reader: EventReader<MovementAction>,
    time: Res<Time>,
    mut player_query: Query<&mut LinearVelocity, With<Player>>,
) {
    let delta_time = time.delta_secs();
    let mut player_velocity = player_query.single_mut();

    for MovementAction(direction) in movement_event_reader.read() {
        player_velocity.x = 10_000. * delta_time * *direction as f32;
    }
}

fn spawn_ball(
    mut ball_spawn_timer: ResMut<BallSpawnTimer>,
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !ball_spawn_timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let mut rng = rand::rng();
    let number = rng.random_range((1 as u32)..30);

    let bound = Bound::random();
    let starting_point = random_point_on_bound(bound);
    let target = random_point_on_bound(bound.other_random());
    let movement_direction = (target - starting_point).normalize();

    commands
        .spawn((
            Numbered(number),
            Ball,
            Mesh2d(meshes.add(Circle::new(number as f32))),
            MeshMaterial2d(materials.add(Color::srgb(1., 0., 0.))),
            Transform::from_translation(starting_point.extend(0.)),
            RigidBody::Kinematic,
            LinearVelocity(movement_direction * 100.),
            Collider::circle(number as f32),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text2d::new(number.to_string()),
                TextFont {
                    font_size: number as f32,
                    ..default()
                },
            ));
        });
}

fn detect_hits(
    player_hits: Query<&CollidingEntities, With<Player>>,
    ball_query: Query<&Numbered, With<Ball>>,
) {
    for hits in player_hits.iter() {
        for hit_entity in hits.iter() {
            if let Ok(Numbered(ball_number)) = ball_query.get(*hit_entity) {
                println!("hit - {ball_number}");
            }
        }
    }
}

fn change_gravity(mut gravity: ResMut<Gravity>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {
        gravity.0 = gravity.0 * -1.;
    }
}

fn despawn_out_of_bounds_balls(
    query: Query<(&Transform, Entity), With<Ball>>,
    mut commands: Commands,
) {
    for (transform, ball_id) in query.iter() {
        if is_out_of_bounds(transform.translation.truncate()) {
            commands.entity(ball_id).despawn_recursive();
        }
    }
}

fn random_point_on_bound(bound: Bound) -> Vec2 {
    let mut rng = rand::rng();

    match bound {
        Bound::UpperBound | Bound::LowerBound => Vec2::new(
            rng.random_range(Bound::LeftBound.value()..Bound::RightBound.value()),
            bound.value(),
        ),
        Bound::RightBound | Bound::LeftBound => Vec2::new(
            bound.value(),
            rng.random_range(Bound::UpperBound.value()..Bound::LowerBound.value()),
        ),
    }
}

fn is_out_of_bounds(point: Vec2) -> bool {
    return point.x < Bound::LeftBound.value()
        || point.x > Bound::RightBound.value()
        || point.y < Bound::UpperBound.value()
        || point.y > Bound::LowerBound.value();
}
