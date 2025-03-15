use avian2d::prelude::*;
use bevy::{prelude::*, time::Stopwatch, window::WindowMode};
use rand::prelude::*;
use std::mem::discriminant;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Wall;

#[derive(Component)]
struct Numbered(i32);

#[derive(Event)]
struct MovementAction(i32);

#[derive(Resource)]
struct BallSpawnTimer(Timer);

#[derive(Resource)]
struct WallBounceStopwatch(Stopwatch);

#[derive(Component)]
struct PlayerText;

const STARTING_NUMBER: i32 = 15;
const SIZE_FACTOR: f32 = 1.5;
const FONT_SIZE_FACTOR: f32 = SIZE_FACTOR * 0.8;

const BACKGROUND_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);

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
            Bound::UpperBound => 500.,
            Bound::LowerBound => -500.,
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
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(WallBounceStopwatch(Stopwatch::new()))
        .add_event::<MovementAction>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                tick_stopwatch,
                keyboard_input,
                change_gravity,
                movement,
                spawn_ball,
                despawn_out_of_bounds_balls,
                handle_hits,
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
            Wall,
            Sprite {
                color: Color::srgb(0.0, 0.4, 0.7),
                custom_size: Some(Vec2::new(size.0, size.1)),
                ..default()
            },
            Transform::from_xyz(transform.0, transform.1, 100.),
            RigidBody::Static,
            Collider::rectangle(size.0, size.1),
            Restitution::PERFECTLY_ELASTIC,
        ));
    }

    let covers = [
        ((10_000., 200.), (0., Bound::UpperBound.value() + 110.)),
        ((10_000., 200.), (0., Bound::LowerBound.value() - 110.)),
        ((200., 10_000.), (Bound::LeftBound.value() - 110., 0.)),
        ((200., 10_000.), (Bound::RightBound.value() + 110., 0.)),
    ];

    for (size, transform) in covers {
        commands.spawn((
            Sprite {
                color: BACKGROUND_COLOR,
                custom_size: Some(Vec2::new(size.0, size.1)),
                ..default()
            },
            Transform::from_xyz(transform.0, transform.1, 99.),
        ));
    }

    commands
        .spawn((
            Player,
            CollidingEntities::default(),
            Numbered(STARTING_NUMBER),
            Mesh2d(meshes.add(Rectangle::new(
                STARTING_NUMBER as f32 * SIZE_FACTOR,
                STARTING_NUMBER as f32 * SIZE_FACTOR,
            ))),
            MeshMaterial2d(materials.add(Color::srgb(0., 0., 1.))),
            Transform::from_xyz(200., 0., 0.),
            RigidBody::Dynamic,
            Restitution::new(0.9),
            Collider::rectangle(
                STARTING_NUMBER as f32 * SIZE_FACTOR,
                STARTING_NUMBER as f32 * SIZE_FACTOR,
            ),
        ))
        .with_children(|builder| {
            builder.spawn((
                PlayerText,
                Text2d::new(STARTING_NUMBER.to_string()),
                TextFont {
                    font_size: STARTING_NUMBER as f32 * FONT_SIZE_FACTOR,
                    ..default()
                },
            ));
        });
}

fn tick_stopwatch(mut stopwatch: ResMut<WallBounceStopwatch>, time: Res<Time>) {
    stopwatch.0.tick(time.delta());
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
    let number = rng.random_range((1 as i32)..100);

    let bound = Bound::random();
    let starting_point = random_point_on_bound(bound);
    let target = random_point_on_bound(bound.other_random());
    let movement_direction = (target - starting_point).normalize();

    commands
        .spawn((
            Numbered(number),
            Ball,
            Mesh2d(meshes.add(Circle::new(number as f32 * SIZE_FACTOR / 2.))),
            MeshMaterial2d(materials.add(Color::srgb(1., 0., 0.))),
            Transform::from_translation(starting_point.extend(0.)),
            RigidBody::Kinematic,
            LinearVelocity(movement_direction * 100.),
            Collider::circle(number as f32 * SIZE_FACTOR / 2.),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text2d::new(number.to_string()),
                TextFont {
                    font_size: number as f32 * FONT_SIZE_FACTOR / 2.,
                    ..default()
                },
            ));
        });
}

fn handle_hits(
    mut player_query: Query<
        (
            &LinearVelocity,
            &CollidingEntities,
            &mut Numbered,
            &mut Collider,
            &mut Mesh2d,
        ),
        With<Player>,
    >,
    mut text_query: Query<(&mut Text2d, &mut TextFont), With<PlayerText>>,
    ball_query: Query<&Numbered, (With<Ball>, Without<Player>)>,
    wall_query: Query<&Wall>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut wall_bounce_stopwatch: ResMut<WallBounceStopwatch>,
) {
    for (player_velocity, hits, mut player_number, mut player_collider, mut player_mesh) in
        player_query.iter_mut()
    {
        for hit_entity in hits.iter() {
            if let Ok(Numbered(ball_number)) = ball_query.get(*hit_entity) {
                let player_number_change = if *ball_number > player_number.0 {
                    -player_number.0 + 1
                } else {
                    commands.spawn(AudioPlayer::new(asset_server.load("sounds/ball_eaten.ogg")));
                    (*ball_number as f32 / 5.).ceil() as i32
                };

                commands.entity(*hit_entity).despawn_recursive();
                player_number.0 += player_number_change;

                let new_size = player_number.0 as f32 * SIZE_FACTOR;
                player_mesh.0 = meshes.add(Rectangle::new(new_size, new_size));
                *player_collider = Collider::rectangle(new_size, new_size);

                let (mut child_text, mut child_text_font) = text_query.single_mut();
                child_text.0 = player_number.0.to_string();
                child_text_font.font_size = player_number.0 as f32 * FONT_SIZE_FACTOR;
            } else if let Ok(_) = wall_query.get(*hit_entity) {
                if player_velocity.length() > 50.
                    && wall_bounce_stopwatch.0.elapsed_secs_f64() > 0.5
                {
                    wall_bounce_stopwatch.0.reset();
                    commands.spawn(AudioPlayer::new(
                        asset_server.load("sounds/wall_bounce.ogg"),
                    ));
                }
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
            rng.random_range(Bound::LowerBound.value()..Bound::UpperBound.value()),
        ),
    }
}

fn is_out_of_bounds(point: Vec2) -> bool {
    return point.x < Bound::LeftBound.value()
        || point.x > Bound::RightBound.value()
        || point.y > Bound::UpperBound.value()
        || point.y < Bound::LowerBound.value();
}
