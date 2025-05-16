use bevy::prelude::{Entity, Vec2, Vec3};
use bevy_quinnet::shared::{
    channels::{ChannelId, ChannelKind, ChannelsConfiguration, DEFAULT_MAX_RELIABLE_FRAME_LEN},
    ClientId,
};
use serde::{Deserialize, Serialize};

use crate::BrickId;

#[derive(Debug, Eq, PartialEq, Clone, Default, Copy, Deserialize, Serialize)]
pub(crate) struct PaddleInputs {
    pub input_ad: PaddleInput,
    pub input_lr: PaddleInput,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub(crate) enum PaddleInput {
    #[default]
    None,
    Left,
    Right,
}

// Messages from clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ClientMessage {
    PaddleInput { input: PaddleInputs },
}

// Messages from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ServerMessage {
    InitClient {
        client_id: ClientId,
    },
    SpawnPaddle {
        owner_client_id: ClientId,
        entity: Entity,
        position: Vec3,
    },
    SpawnBall {
        owner_client_id: ClientId,
        entity: Entity,
        position: Vec3,
        direction: Vec2,
    },
    StartGame,
    BrickDestroyed {
        by_client_id: ClientId,
        brick_id: BrickId,
    },
    Scored {
        by_client_id: ClientId,
    },
    // SetScore {
    //     score_a: i32,
    //     score_b: i32,
    // },
    BallCollided {
        owner_client_id: ClientId,
        entity: Entity,
        position: Vec3,
        velocity: Vec2,
    },
    PaddleMoved {
        entity: Entity,
        position: Vec3,
    },
}

#[repr(u8)]
pub enum ClientChannel {
    PaddleCommands,
}
impl Into<ChannelId> for ClientChannel {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}
impl ClientChannel {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![ChannelKind::default()]).unwrap()
    }
}

#[repr(u8)]
pub enum ServerChannel {
    GameSetup,
    GameEvents,
    PaddleUpdates,
}
impl Into<ChannelId> for ServerChannel {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}
impl ServerChannel {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![
            ChannelKind::OrderedReliable {
                max_frame_size: DEFAULT_MAX_RELIABLE_FRAME_LEN,
            },
            ChannelKind::UnorderedReliable {
                max_frame_size: DEFAULT_MAX_RELIABLE_FRAME_LEN,
            },
            ChannelKind::Unreliable,
        ])
        .unwrap()
    }
}
