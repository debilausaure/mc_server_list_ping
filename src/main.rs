#![warn(rust_2018_idioms)]

use tokio::io::{BufStream, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use mc_server_list_ping::*;

#[tokio::main]
async fn main() -> Result<(), AsyncError> {
    let mut listener = TcpListener::bind("127.0.0.1:25565").await?;

    while let Ok((client_stream, _)) = listener.accept().await {
        tokio::spawn(fsm(client_stream));
    }

    Ok(())
}

async fn fsm(client_stream: TcpStream) -> Result<(), AsyncError> {
    let mut client_stream = BufStream::new(client_stream);
    let (handshake_packet, _) = parse_handshake_packet(&mut client_stream).await?;
    println!("{:?}", handshake_packet);
    let (request_packet, _) = parse_request_packet(&mut client_stream).await?;
    println!("{:?}", request_packet);
    write_response_packet(&mut client_stream, "1.15.2", 578, 0, 0, "Server not started").await?;
    let (ping_packet, _) = parse_ping_packet(&mut client_stream).await?;
    println!("{:?}", ping_packet);
    write_pong_packet(&mut client_stream, match ping_packet.data {
        PacketData::Ping(n) => n,
        _ => return Err("Not a ping packet !".into()),
    }).await?;
    Ok(())
}

async fn print_bytes(mut client_stream: TcpStream) -> Result<(), AsyncError> {
    loop {
        match client_stream.read_u8().await {
            Ok(byte) => print!("{:x} ", byte),
            _ => {
                println!();
                return Ok(());
            }
        }
    }
}
