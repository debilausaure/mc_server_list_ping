#![warn(rust_2018_idioms)]


use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

async fn fsm(mut client_stream: TcpStream) -> Result<(), AsyncError> {
    let (handshake_packet, _) = parse_handshake_packet(&mut client_stream).await?;
    println!("{:?}", handshake_packet);
    let (request_packet, _) = parse_request_packet(&mut client_stream).await?;
    println!("{:?}", request_packet);
    response(client_stream).await
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

async fn response(mut client_stream: TcpStream) -> Result<(), AsyncError> {
    let json = br#"{
    "version": {
        "name": "1.15.2",
        "protocol": 578
    },
    "players": {
        "max": 100,
        "online": 5,
        "sample": [{}]
    },  
    "description": {
        "text": "A fake server"
    },
}"#;
    dbg!(json.len());
    write_varInt(&mut client_stream, json.len() as i32 + 2)
        .await
        .unwrap();
    write_varInt(&mut client_stream, 0).await.unwrap();
    write_varInt(&mut client_stream, json.len() as i32)
        .await
        .unwrap();
    client_stream.write_all(json).await.unwrap();
    Ok(())
}
