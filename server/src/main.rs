use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use rhythm_game_server::protocol::{ClientboundPacket, NetError, ServerboundPacket, recv, send};

// runs on a seperate thread for each client
fn recv_loop(stream: TcpStream) {
    println!("opened a new connection: {:?}", stream);
    // stream
    //     .set_read_timeout(Some(Duration::from_secs(10)))
    //     .expect("Failed to set read timeout");
    loop {
        match recv::<ServerboundPacket>(&mut &stream) {
            Ok(packet) => {
                println!("recv: {:?}", packet);
                // send a pong back
                if let Err(e) = send(ClientboundPacket::Pong, &mut &stream) {
                    eprintln!("failed to send pong: {e}");
                }
            }
            Err(e) => {
                eprintln!("recv failed: {e}");
                if let Err(e) = send(ClientboundPacket::Error(NetError::Malformed), &mut &stream) {
                    eprintln!("failed to send error: {e}");
                }
                break;
            }
        }
    }
    println!("connection dropped");
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:7300")?;
    println!("listening on 0.0.0.0:7300");
    for stream in listener.incoming() {
        match stream {
            Ok(s) => thread::spawn(move || {
                recv_loop(s);
            }),
            Err(e) => {
                eprintln!("connection failed: {e}");
                continue;
            }
        };
    }
    Ok(())
}
