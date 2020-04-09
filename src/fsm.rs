use crate::types::*;
use crate::AsyncError;

use std::convert::{TryInto};
use tokio::io::{AsyncRead, AsyncWrite};


pub struct Fsm<'a> {
    version_name: &'a str,
    version_protocol: i32,
    player_max: usize,
    player_online: usize,
    description: &'a str,
}

impl<'a> Fsm<'a> {
    pub fn new(version_name: &'a str, version_protocol: i32) -> Self {
        Self {
            version_name,
            version_protocol,
            player_max: 0,
            player_online: 0,
            description: "A minecraft server",
        }
    }

    pub fn player_max(self, player_max: usize) -> Self {
        Self { player_max, ..self }
    }

    pub fn player_online(self, player_online: usize) -> Self {
        Self {
            player_online,
            ..self
        }
    }

    pub fn description(self, description: &'a str) -> Self {
        Self {
            description,
            ..self
        }
    }

    pub async fn run<RW: AsyncRead + AsyncWrite + Unpin>(
        self,
        mut client_stream: RW,
    ) -> Result<Option<(RW, HandshakePacket)>, AsyncError> {
        let (handshake_packet, _) = HandshakePacket::parse(&mut client_stream).await?;
        match handshake_packet.next_state.try_into()? {
            1_usize => (),
            2 => return Ok(Some((client_stream, handshake_packet))),
            _ => return Err("Unexpected next_state".into()),
        }
        RequestPacket::parse(&mut client_stream).await?;
        let response_packet = ResponsePacket::new(
            self.version_name,
            self.version_protocol,
            self.player_max,
            self.player_online,
            self.description,
        );
        response_packet.send(&mut client_stream).await?;
        let (ping_packet, _) = PingPacket::parse(&mut client_stream).await?;
        ping_packet.send(&mut client_stream).await?;
        Ok(None)
    }
}
