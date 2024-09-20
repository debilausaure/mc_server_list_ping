#![warn(rust_2018_idioms)]

use mc_server_list_ping::*;
use tokio::{
    io::BufStream,
    net::{TcpListener, TcpStream},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AsyncError> {
    let listener = TcpListener::bind("127.0.0.1:25565").await?;
    while let Ok((client_stream, _)) = listener.accept().await {
        tokio::spawn(fsm(client_stream));
    }
    Ok(())
}

async fn fsm(client_stream: TcpStream) -> Result<(), AsyncError> {
    let client_stream = BufStream::new(client_stream);
    let fsm = Fsm::new("1.21.1", 767).description("Server not started");
    match fsm.run(client_stream).await? {
        None => println!("Server list ping"),
        Some((_, packet)) => println!("{:?}", packet),
    }
    Ok(())
}
