use std::{collections::HashMap, ops::DerefMut};

use bevy::{
    asset::Assets,
    audio::{AudioPlayer, Volume},
    ecs::system::Single,
    input::ButtonInput,
    prelude::{
        default, AssetServer, Bundle, Button, Camera2d, Changed, Circle, Color, Commands,
        Component, Entity, EventReader, EventWriter, KeyCode, Local, Mesh, Mesh2d,
        PlaybackSettings, Query, Res, ResMut, Resource, Text, Transform, Vec2, Vec3, With, Without,
    },
    sprite::{ColorMaterial, MeshMaterial2d, Sprite},
    state::state::NextState,
    text::{TextColor, TextFont, TextSpan},
    ui::{
        AlignItems, BackgroundColor, Interaction, JustifyContent, Node, PositionType, UiRect, Val,
    },
};
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode, connection::ClientEndpointConfiguration,
        QuinnetClient,
    },
    shared::ClientId,
};

use crate::{
    protocol::{ClientChannel, ClientMessage, PaddleInput, PaddleInputs, ServerMessage},
    BrickId, CollisionEvent, CollisionSound, GameState, Score, Velocity, WallLocation, BALL_SIZE,
    BALL_SPEED, BRICK_SIZE, GAP_BETWEEN_BRICKS, LOCAL_BIND_IP, PADDLE_SIZE, SERVER_HOST,
    SERVER_PORT, TIME_STEP,
};

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

pub(crate) const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const PADDLE_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const OPPONENT_PADDLE_COLOR: Color = Color::srgb(1.0, 0.5, 0.5);
const BALL_COLOR: Color = Color::srgb(0.35, 0.35, 0.6);
const OPPONENT_BALL_COLOR: Color = Color::srgb(0.9, 0.6, 0.6);
const BRICK_COLOR: Color = Color::srgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::srgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::srgb(1.0, 0.5, 0.5);
const NORMAL_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON_COLOR: Color = Color::srgb(0.35, 0.75, 0.35);
const BUTTON_TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

const BOLD_FONT: &str = "fonts/FiraSans-Bold.ttf";
const NORMAL_FONT: &str = "fonts/FiraMono-Medium.ttf";
const COLLISION_SOUND_EFFECT: &str = "sounds/breakout_collision.ogg";

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct ClientData {
    self_id: ClientId,
}

#[derive(Resource, Default)]
pub(crate) struct NetworkMapping {
    // Network entity id to local entity id
    map: HashMap<Entity, Entity>,
}

// This resource tracks the game's score
#[derive(Resource)]
pub(crate) struct Scoreboard {
    pub(crate) score_a: i32,
    pub(crate) score_b: i32,
}

#[derive(Component)]
pub(crate) struct Paddle;

#[derive(Component)]
pub(crate) struct Ball;

#[derive(Component)]
pub(crate) struct Brick;

/// The buttons in the main menu.
#[derive(Clone, Copy, Component)]
pub(crate) enum MenuItem {
    Host,
    Join,
}

#[derive(Bundle)]
struct WallBundle {
    sprite: Sprite,
    transform: Transform,
}
pub(crate) fn start_connection(mut client: ResMut<QuinnetClient>) {
    client
        .open_connection(
            ClientEndpointConfiguration::from_ips(SERVER_HOST, SERVER_PORT, LOCAL_BIND_IP, 0),
            CertificateVerificationMode::SkipVerification,
            ClientChannel::channels_configuration(),
        )
        .unwrap();
}

fn spawn_paddle(commands: &mut Commands, position: &Vec3, owned: bool) -> Entity {
    commands
        .spawn((
            Sprite::from_color(
                if owned {
                    PADDLE_COLOR
                } else {
                    OPPONENT_PADDLE_COLOR
                },
                Vec2::ONE,
            ),
            Transform {
                translation: *position,
                scale: PADDLE_SIZE,
                ..default()
            },
            Paddle,
        ))
        .id()
}

pub(crate) fn handle_server_messages(
    mut commands: Commands,
    mut client: ResMut<QuinnetClient>,
    mut client_data: ResMut<ClientData>,
    mut entity_mapping: ResMut<NetworkMapping>,
    mut next_state: ResMut<NextState<GameState>>,
    mut paddles: Query<&mut Transform, With<Paddle>>,
    mut balls: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut MeshMaterial2d<ColorMaterial>,
        ),
        (With<Ball>, Without<Paddle>),
    >,
    mut scoreboard: ResMut<Scoreboard>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    while let Some((_, message)) = client
        .connection_mut()
        .try_receive_message::<ServerMessage>()
    {
        match message {
            ServerMessage::InitClient { client_id } => {
                client_data.self_id = client_id;
            }
            ServerMessage::SpawnPaddle {
                owner_client_id,
                entity,
                position,
            } => {
                let paddle = spawn_paddle(
                    &mut commands,
                    &position,
                    owner_client_id == client_data.self_id,
                );
                entity_mapping.map.insert(entity, paddle);
            }
            ServerMessage::SpawnBall {
                owner_client_id,
                entity,
                position,
                direction,
            } => {
                let ball = commands
                    .spawn((
                        Mesh2d(meshes.add(Circle::default())),
                        MeshMaterial2d(materials.add(player_color_from_bool(
                            owner_client_id == client_data.self_id,
                        ))),
                        Transform::from_translation(position).with_scale(BALL_SIZE),
                        Ball,
                        Velocity(direction.normalize() * BALL_SPEED),
                    ))
                    .id();
                entity_mapping.map.insert(entity, ball);
            }
            ServerMessage::StartGame {} => next_state.set(GameState::Running),
            ServerMessage::BrickDestroyed {
                by_client_id,
                brick_id,
            } => {
                if by_client_id == client_data.self_id {
                    scoreboard.score_a += 1;
                } else {
                    scoreboard.score_b += 1;
                }
            }
            ServerMessage::Scored { by_client_id } => {
                // if by_client_id == client_data.self_id {
                if by_client_id == 0 {
                    scoreboard.score_a += 1;
                } else {
                    scoreboard.score_b += 1;
                }
            }
            ServerMessage::BallCollided {
                owner_client_id,
                entity,
                position,
                velocity,
            } => {
                if let Some(local_ball) = entity_mapping.map.get(&entity) {
                    if let Ok((mut ball_transform, mut ball_velocity, mut ball_mat)) =
                        balls.get_mut(*local_ball)
                    {
                        ball_transform.translation = position;
                        ball_velocity.0 = velocity;
                        ball_mat.0 = materials.add(player_color_from_bool(
                            owner_client_id == client_data.self_id,
                        ));
                    }
                }
                // Sends a collision event so that other systems can react to the collision
                collision_events.write_default();
            }
            ServerMessage::PaddleMoved { entity, position } => {
                if let Some(local_paddle) = entity_mapping.map.get(&entity) {
                    if let Ok(mut paddle_transform) = paddles.get_mut(*local_paddle) {
                        paddle_transform.translation = position;
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct PaddleState {
    current_input: PaddleInputs,
}

pub(crate) fn move_paddle(
    mut client: ResMut<QuinnetClient>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut local: Local<PaddleState>,
) {
    let mut paddle_input = PaddleInputs {
        input_ad: PaddleInput::None,
        input_lr: PaddleInput::None,
    };

    if keyboard_input.pressed(KeyCode::KeyA) {
        paddle_input.input_ad = PaddleInput::Left;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        paddle_input.input_ad = PaddleInput::Right;
    }

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        paddle_input.input_lr = PaddleInput::Left;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        paddle_input.input_lr = PaddleInput::Right;
    }

    if local.current_input != paddle_input {
        client.connection_mut().try_send_message_on(
            ClientChannel::PaddleCommands,
            ClientMessage::PaddleInput {
                input: paddle_input,
            },
        );
        local.current_input = paddle_input;
    }
}

pub(crate) fn update_scoreboard(
    scoreboard: Res<Scoreboard>,
    mut query: Single<(&mut TextSpan, &mut TextColor), With<Score>>,
) {
    let (ref mut text, ref mut text_color) = query.deref_mut();
    text.0 = format!("{}-{}", scoreboard.score_a, scoreboard.score_b);
    text_color.0 = Color::BLACK;
}

pub(crate) fn play_collision_sound(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    sound: Res<CollisionSound>,
) {
    // Play a sound once per frame if a collision occurred.
    if !collision_events.is_empty() {
        // This prevents events staying active on the next frame.
        collision_events.clear();
        commands.spawn((
            AudioPlayer(sound.clone()),
            // auto-despawn the entity when playback finishes
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.03)),
        ));
    }
}

pub(crate) fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2d::default());

    let button_style = Node {
        width: Val::Px(150.0),
        height: Val::Px(65.0),
        // center button
        margin: UiRect::all(Val::Auto),
        // horizontally center child text
        justify_content: JustifyContent::Center,
        // vertically center child text
        align_items: AlignItems::Center,
        ..default()
    };

    let text_font = TextFont {
        font: asset_server.load(BOLD_FONT),
        font_size: 40.,
        ..Default::default()
    };
    let text_color = TextColor(BUTTON_TEXT_COLOR);
    commands
        .spawn((
            Node {
                width: Val::Vw(100.0),
                height: Val::Vh(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE.into()),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    button_style.clone(),
                    Button,
                    BackgroundColor(NORMAL_BUTTON_COLOR.into()),
                    MenuItem::Host,
                ))
                .with_children(|parent| {
                    parent.spawn((Text("Host".into()), text_font.clone(), text_color.clone()));
                });
            parent
                .spawn((
                    button_style.clone(),
                    Button,
                    BackgroundColor(NORMAL_BUTTON_COLOR.into()),
                    MenuItem::Join,
                ))
                .with_children(|parent| {
                    parent.spawn((Text("Join".into()), text_font.clone(), text_color.clone()));
                });
        });
}

pub(crate) fn handle_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &MenuItem),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, item) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON_COLOR.into();
                match item {
                    MenuItem::Host => next_state.set(GameState::HostingLobby),
                    MenuItem::Join => next_state.set(GameState::JoiningLobby),
                }
            }
            Interaction::Hovered => *color = HOVERED_BUTTON_COLOR.into(),
            Interaction::None => *color = NORMAL_BUTTON_COLOR.into(),
        }
    }
}

pub(crate) fn teardown_main_menu(mut commands: Commands, query: Query<Entity, With<Button>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn setup_breakout(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Sound
    let ball_collision_sound = asset_server.load(COLLISION_SOUND_EFFECT);
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Scoreboard
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: SCOREBOARD_TEXT_PADDING,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
            Text("Score: ".into()),
            TextFont {
                font: asset_server.load(BOLD_FONT),
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_COLOR),
        ))
        .with_child((
            TextSpan("".into()),
            TextFont {
                font: asset_server.load(NORMAL_FONT),
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(SCORE_COLOR),
            Score,
        ));

    // Walls
    commands.spawn(WallBundle::new(WallLocation::Left));
    commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Bottom));
    commands.spawn(WallBundle::new(WallLocation::Top));
}

pub(crate) fn apply_velocity(mut query: Query<(&mut Transform, &Velocity), With<Ball>>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
    }
}

fn player_color_from_bool(owned: bool) -> Color {
    if owned {
        BALL_COLOR
    } else {
        OPPONENT_BALL_COLOR
    }
}

impl WallBundle {
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
            sprite: Sprite::from_color(WALL_COLOR, Vec2::ONE),
            transform: Transform {
                // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                // This is used to determine the order of our sprites
                translation: location.position().extend(0.0),
                // The z-scale of 2D objects must always be 1.0,
                // or their ordering will be affected in surprising ways.
                // See https://github.com/bevyengine/bevy/issues/4149
                scale: location.size().extend(1.0),
                ..default()
            },
        }
    }
}
