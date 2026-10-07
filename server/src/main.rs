use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use rhythm_game_server::protocol::{ClientboundPacket, NetError, ServerboundPacket};

fn recv(stream: &mut impl Read) -> Result<ServerboundPacket, Box<dyn Error>> {
    // read length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf);
    println!("recv: len {}", len);
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf)?;
    Ok(rmp_serde::from_slice(&buf)?)
}

fn send(payload: ClientboundPacket, stream: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let payload = rmp_serde::to_vec(&payload)?;
    let len = payload.len() as u32;
    let len_bytes = len.to_be_bytes();
    stream.write_all(&len_bytes)?;
    stream.write_all(&payload)?;
    Ok(())
}

// runs on a seperate thread for each client
fn recv_loop(stream: TcpStream) {
    println!("opened a new connection: {:?}", stream);
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("Failed to set read timeout");
    match recv(&mut &stream) {
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
