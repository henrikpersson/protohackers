use std::{net::{TcpListener, TcpStream}, io::{BufReader, BufWriter, Read, Write}};

use async_std::task;

fn host() -> &'static str {
  #[cfg(debug_assertions)]
  return "127.0.0.1:4444";
  #[cfg(not(debug_assertions))]
  return "206.189.10.100:4444";
}

async fn handle_client(stream: &TcpStream) -> Result<usize, Box<dyn std::error::Error>> {
  let mut read = BufReader::new(stream);
  let mut write = BufWriter::new(stream);
  let mut buf: Vec<u8> = Vec::with_capacity(1024);

  let read = read.read_to_end(&mut buf)?;
  write.write_all(&buf)?;
  write.flush().unwrap();
  Ok(read)
} 

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let server = TcpListener::bind(host())?;

  for client in server.incoming() {
    task::spawn(async {
      if let Ok(stream) = client {
        println!("client connected!");
        let res = handle_client(&stream).await;
        println!("client disconnected...");

        match res {
          Ok(bytes_read) => println!("read {} bytes from a client", bytes_read),
          Err(err) => println!("client errored: {}", err)
        }
      }
    });
  }

  Ok(())
}
