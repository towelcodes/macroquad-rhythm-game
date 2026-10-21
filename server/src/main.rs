use std::{
    error::Error,
    io::{Read, Write},
    mem,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

use rhythm_game_server::protocol::{ClientboundPacket, NetError, ServerboundPacket, recv, send};

use crate::db::DatabaseController;

#[cfg(test)]
mod test;

mod db;

// runs on a seperate thread for each client
fn recv_loop(stream: TcpStream, con: Arc<Mutex<DatabaseController>>) {
    println!("opened a new connection: {:?}", stream);
    loop {
        match recv::<ServerboundPacket>(&mut &stream) {
            Ok(packet) => {
                println!("recv: {:?}", packet);
                match packet {
                    ServerboundPacket::Ping(id) => {
                        // send a pong back
                        if let Err(e) = send(ClientboundPacket::Pong(id), &mut &stream) {
                            eprintln!("failed to send pong: {e}");
                        }
                    }
                    ServerboundPacket::Register {
                        id,
                        username,
                        password,
                    } => {
                        // create the user
                        // FIXME this blocks, will cause bad performance on high load
                        let mut lock = con.lock().unwrap();
                        match lock.register_user(username.clone(), password) {
                            Ok((uid, token)) => {
                                mem::drop(lock); // drop the lock first so we don't wait for send
                                if let Err(e) = send(
                                    ClientboundPacket::LoggedIn {
                                        id,
                                        uid,
                                        username,
                                        token,
                                    },
                                    &mut &stream,
                                ) {
                                    eprintln!("failed to send response: {e}");
                                }
                            }
                            Err(why) => {
                                mem::drop(lock);
                                eprintln!("failed to register user: {why}");
                                if let Err(e) = send(
                                    ClientboundPacket::Error(0, NetError::Malformed),
                                    &mut &stream,
                                ) {
                                    eprintln!("failed to send error: {e}");
                                }
                            }
                        }
                    }
                    ServerboundPacket::Login {
                        username,
                        password,
                        id,
                    } => {
                        let mut lock = con.lock().unwrap();
                        match lock.login_user(username.clone(), password) {
                            Ok((uid, token)) => {
                                mem::drop(lock);
                                if let Err(e) = send(
                                    ClientboundPacket::LoggedIn {
                                        id,
                                        uid,
                                        username,
                                        token,
                                    },
                                    &mut &stream,
                                ) {
                                    eprintln!("failed to send response: {e}");
                                }
                            }
                            Err(why) => {
                                mem::drop(lock);
                                eprintln!("failed to login user: {why}");
                                if let Err(e) =
                                    send(ClientboundPacket::AuthInvalid(id), &mut &stream)
                                {
                                    eprintln!("failed to send error: {e}");
                                }
                            }
                        }
                    }
                    _ => {
                        println!("packet not implemented! no response will be given");
                    }
                }
            }
            Err(e) => {
                eprintln!("recv failed: {e}");
                if let Err(e) = send(
                    ClientboundPacket::Error(0, NetError::Malformed),
                    &mut &stream,
                ) {
                    eprintln!("failed to send error: {e}");
                }
                break;
            }
        }
    }
    println!("connection dropped");
}

fn main() -> std::io::Result<()> {
    // load or create the database
    // we use an arc/mutex so we can share it across threads
    //  - in a production environment a connection pool
    //    or other alternative method should be used for performance
    let con = Arc::new(Mutex::new(
        DatabaseController::new("server.db").expect("failed to open database"),
    ));
    Arc::clone(&con)
        .lock()
        .unwrap()
        .perform_migrations()
        .expect("failed to perform migrations");

    let listener = TcpListener::bind("0.0.0.0:7300")?;
    println!("listening on 0.0.0.0:7300");
    for stream in listener.incoming() {
        let con2 = Arc::clone(&con);
        match stream {
            Ok(s) => thread::spawn(move || {
                recv_loop(s, con2);
            }),
            Err(e) => {
                eprintln!("connection failed: {e}");
                continue;
            }
        };
    }
    Ok(())
}
