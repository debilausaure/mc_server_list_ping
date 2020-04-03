use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub type AsyncError = Box<dyn std::error::Error + Send + Sync>;

#[allow(non_snake_case)]
pub async fn parse_varInt(client_stream: &mut TcpStream) -> Result<(i32, usize), AsyncError> {
    let mut read_int: i32 = 0;
    let mut bytes_read: usize = 0;
    loop {
        let incoming_byte = client_stream.read_u8().await?;
        read_int |= ((incoming_byte & 0b0111_1111) as i32) << 7 * bytes_read;
        bytes_read += 1;
        if incoming_byte >> 7 == 0 {
            return Ok((read_int, bytes_read));
        } else if bytes_read == 5 {
            return Err("VarInt bigger than 5 bytes sent".into());
        }
    }
}

#[allow(non_snake_case)]
pub async fn write_varInt(client_stream: &mut TcpStream, mut n: i32) -> Result<usize, AsyncError> {
    let mut bytes_sent = 0;
    loop {
        bytes_sent += 1;
        let tmp = n as u8 & 0b0111_1111;
        n >>= 7;
        if n == 0 {
            client_stream.write_u8(tmp).await?;
            break;
        } else {
            client_stream.write_u8(tmp | 0b1000_0000).await?;
        }
    }

    Ok(bytes_sent)
}

#[allow(non_snake_case)]
pub async fn parse_String(client_stream: &mut TcpStream) -> Result<(String, usize), AsyncError> {
    let (string_size, bytes_read) = parse_varInt(client_stream).await?;
    let mut string = String::new();
    client_stream
        .take(string_size as u64)
        .read_to_string(&mut string)
        .await?;
    Ok((string, string_size as usize + bytes_read))
}

#[allow(non_snake_case)]
pub async fn parse_unsignedShort(
    client_stream: &mut TcpStream,
) -> Result<(u16, usize), AsyncError> {
    Ok((client_stream.read_u16().await?, 2))
}

#[derive(Debug)]
pub struct HandshakeData {
    protocol_version: i32,
    server_address: String,
    server_port: u16,
    next_state: i32,
}

#[derive(Debug)]
pub enum PacketData {
    Handshake(Box<HandshakeData>),
    Request,
}

#[derive(Debug)]
pub struct Packet {
    length: i32,
    packet_id: i32,
    data: PacketData,
}

pub async fn parse_handshake_packet(
    client_stream: &mut TcpStream,
) -> Result<(Packet, usize), AsyncError> {
    let handshake_packet = Packet {
        length: parse_varInt(client_stream).await?.0,
        packet_id: parse_varInt(client_stream).await?.0,
        data: parse_handshake_data(client_stream).await?.0,
    };
    let packet_length = handshake_packet.length as usize;
    Ok((handshake_packet, packet_length))
}

pub async fn parse_handshake_data(
    client_stream: &mut TcpStream,
) -> Result<(PacketData, usize), AsyncError> {
    let mut bytes_read = 0;
    let (protocol_version, tmp_bytes_read) = parse_varInt(client_stream).await?;
    bytes_read += tmp_bytes_read;
    let (server_address, tmp_bytes_read) = parse_String(client_stream).await?;
    bytes_read += tmp_bytes_read;
    let (server_port, tmp_bytes_read) = parse_unsignedShort(client_stream).await?;
    bytes_read += tmp_bytes_read;
    let (next_state, tmp_bytes_read) = parse_varInt(client_stream).await?;
    bytes_read += tmp_bytes_read;
    Ok((
        PacketData::Handshake(Box::new(HandshakeData {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })),
        bytes_read,
    ))
}

pub async fn parse_request_packet(
    client_stream: &mut TcpStream,
) -> Result<(Packet, usize), AsyncError> {
    let request_packet = Packet {
        length: parse_varInt(client_stream).await?.0,
        packet_id: parse_varInt(client_stream).await?.0,
        data: PacketData::Request,
    };
    let packet_length = request_packet.length as usize;
    Ok((request_packet, packet_length))
}
