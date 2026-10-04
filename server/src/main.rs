use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:7300")?;
    println!("listening on 0.0.0.0:7300");
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("connection failed: {e}");
                continue;
            }
        };
    }
    Ok(())
}
