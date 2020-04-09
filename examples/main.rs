#![warn(rust_2018_idioms)]

use tokio::io::{BufStream};
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
    let client_stream = BufStream::new(client_stream);
    let fsm = Fsm::new(
        "1.15.2",
        578,
    ).description(
        "Server not started",
    );
    match fsm.run(client_stream).await? {
        None => println!("Server list ping"),
        Some((_ , packet)) => println!("{:?}", packet),
    }
    Ok(())
}
