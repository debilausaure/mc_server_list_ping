use std::convert::{TryFrom, TryInto};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::AsyncError;

#[derive(Copy, Clone, Debug)]
pub struct VarInt {
    inner: i32,
}

impl VarInt {
    pub fn from(n: i32) -> Self {
        Self { inner: n }
    }

    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let mut read_int: i32 = 0;
        let mut bytes_read: usize = 0;
        loop {
            let incoming_byte = readable_stream.read_u8().await?;
            read_int |= ((incoming_byte & 0b0111_1111) as i32) << 7 * bytes_read;
            bytes_read += 1;
            if incoming_byte >> 7 == 0 {
                return Ok((Self { inner: read_int }, bytes_read));
            } else if bytes_read == 5 {
                return Err("VarInt bigger than 5 bytes sent".into());
            }
        }
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut n = self.inner as u32;
        let mut bytes_sent = 0;
        loop {
            bytes_sent += 1;
            let tmp = n as u8 & 0b0111_1111;
            n >>= 7;
            if n == 0 {
                writable_stream.write_u8(tmp).await?;
                break;
            } else {
                writable_stream.write_u8(tmp | 0b1000_0000).await?;
            }
        }
        Ok(bytes_sent)
    }
}

macro_rules! ImplTryFrom {
    ($prim:ty) => {
        impl TryFrom<VarInt> for $prim {
            type Error = AsyncError;
            fn try_from(f: VarInt) -> Result<Self, Self::Error> {
                Ok(f.inner.try_into()?)
            }
        }

        impl TryFrom<$prim> for VarInt {
            type Error = AsyncError;
            fn try_from(ty: $prim) -> Result<Self, Self::Error> {
                Ok(Self {
                    inner: ty.try_into()?,
                })
            }
        }
    };
}

ImplTryFrom!(u64);
ImplTryFrom!(usize);

#[derive(Debug)]
struct String {
    inner: std::string::String,
}

impl String {
    pub fn new(s: std::string::String) -> Self {
        Self { inner: s }
    }

    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let (string_size, bytes_read) = VarInt::parse(readable_stream).await?;
        let string_size: u64 = string_size.try_into()?;
        let mut string = std::string::String::new();
        readable_stream
            .take(string_size)
            .read_to_string(&mut string)
            .await?;
        Ok((Self { inner: string }, bytes_read + string_size as usize))
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut bytes_sent = 0;
        bytes_sent += VarInt::from(self.inner.len().try_into()?)
            .send(writable_stream)
            .await?;
        writable_stream.write_all(self.inner.as_bytes()).await?;
        bytes_sent += self.inner.len();
        Ok(bytes_sent)
    }
}

#[allow(non_snake_case)]
pub async fn parse_UnsignedShort<R: AsyncRead + Unpin>(
    readable_stream: &mut R,
) -> Result<(u16, usize), AsyncError> {
    Ok((readable_stream.read_u16().await?, 2))
}

#[allow(non_snake_case)]
pub async fn send_UnsignedShort<W: AsyncWrite + Unpin>(
    writable_stream: &mut W,
    u: u16,
) -> Result<usize, AsyncError> {
    writable_stream.write_u16(u).await?;
    Ok(2)
}

#[allow(non_snake_case)]
pub async fn parse_Long<R: AsyncRead + Unpin>(
    readable_stream: &mut R,
) -> Result<(i64, usize), AsyncError> {
    Ok((readable_stream.read_i64().await?, 8))
}

#[allow(non_snake_case)]
pub async fn send_Long<W: AsyncWrite + Unpin>(
    writable_stream: &mut W,
    i: i64,
) -> Result<usize, AsyncError> {
    writable_stream.write_i64(i).await?;
    Ok(8)
}

#[derive(Debug)]
pub struct Header {
    length: VarInt,
    packet_id: VarInt,
}

impl Header {
    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let (length, bytes_read) = VarInt::parse(readable_stream).await?;
        let (packet_id, bytes_read_tmp) = VarInt::parse(readable_stream).await?;
        let header = Self { length, packet_id };
        Ok((header, bytes_read + bytes_read_tmp))
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut bytes_sent = 0;
        bytes_sent += self.length.send(writable_stream).await?;
        bytes_sent += self.packet_id.send(writable_stream).await?;
        Ok(bytes_sent)
    }

    pub fn new(length: VarInt, packet_id: VarInt) -> Self {
        Self { length, packet_id }
    }
}

#[derive(Debug)]
pub struct HandshakePacket {
    header: Header,
    protocol_version: VarInt,
    server_address: String,
    server_port: u16,
    pub next_state: VarInt,
}

impl HandshakePacket {
    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let (header, mut bytes_read) = Header::parse(readable_stream).await?;
        let (protocol_version, tmp_bytes_read) = VarInt::parse(readable_stream).await?;
        bytes_read += tmp_bytes_read;
        let (server_address, tmp_bytes_read) = String::parse(readable_stream).await?;
        bytes_read += tmp_bytes_read;
        let (server_port, tmp_bytes_read) = parse_UnsignedShort(readable_stream).await?;
        bytes_read += tmp_bytes_read;
        let (next_state, tmp_bytes_read) = VarInt::parse(readable_stream).await?;
        bytes_read += tmp_bytes_read;
        Ok((
            Self {
                header,
                protocol_version,
                server_address,
                server_port,
                next_state,
            },
            bytes_read,
        ))
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut bytes_sent = 0;
        bytes_sent += self.header.send(writable_stream).await?;
        bytes_sent += self.protocol_version.send(writable_stream).await?;
        bytes_sent += self.server_address.send(writable_stream).await?;
        bytes_sent += send_UnsignedShort(writable_stream, self.server_port).await?;
        bytes_sent += self.next_state.send(writable_stream).await?;
        writable_stream.flush().await?;
        Ok(bytes_sent)
    }
}

pub struct RequestPacket {
    _header: Header,
}

impl RequestPacket {
    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let (_header, bytes_read) = Header::parse(readable_stream).await?;
        Ok((Self { _header }, bytes_read))
    }
}

pub struct PingPacket {
    header: Header,
    ping_id: i64,
}

impl PingPacket {
    pub async fn parse<R: AsyncRead + Unpin>(
        readable_stream: &mut R,
    ) -> Result<(Self, usize), AsyncError> {
        let (header, bytes_read) = Header::parse(readable_stream).await?;
        let (ping_id, tmp_bytes_read) = parse_Long(readable_stream).await?;
        Ok((Self { header, ping_id }, bytes_read + tmp_bytes_read))
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut bytes_sent = 0;
        bytes_sent += self.header.send(writable_stream).await?;
        bytes_sent += send_Long(writable_stream, self.ping_id).await?;
        writable_stream.flush().await?;
        Ok(bytes_sent)
    }
}

pub struct ResponsePacket {
    header: Header,
    json: String,
}

impl ResponsePacket {
    pub fn new(
        version_name: &str,
        version_protocol: i32,
        player_max: usize,
        player_online: usize,
        description: &str,
    ) -> Self {
        let json = format!(
            r#"{{"version":{{"name":"{version_name}","protocol":{version_protocol}}},"players":{{"max":{player_max},"online":{player_online}}},"description":{{"text":"{description}"}}}}"#,
            version_name = version_name,
            version_protocol = version_protocol,
            player_max = player_max,
            player_online = player_online,
            description = description
        );
        let string_size = json.len();
        let json = String::new(json);
        let header = Header::new(VarInt::from(string_size as i32 + 2), VarInt::from(0)); //FIXME
        Self { header, json }
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self,
        writable_stream: &mut W,
    ) -> Result<usize, AsyncError> {
        let mut bytes_sent = 0;
        bytes_sent += self.header.send(writable_stream).await?;
        bytes_sent += self.json.send(writable_stream).await?;
        writable_stream.flush().await?;
        Ok(bytes_sent)
    }
}
