use std::{convert::TryInto, collections::HashMap};
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};

const MESSAGE_LEN: usize = 9;

fn host() -> &'static str {
  #[cfg(debug_assertions)]
  return "127.0.0.1:4444";
  #[cfg(not(debug_assertions))]
  return "206.189.10.100:4444";
}

#[derive(Debug)]
enum Error {
  UndefinedBehavior
}

enum Message {
  Insert { timestamp: i32, price: i32 },
  Query { mintime: i32, maxtime: i32 }
}

impl Message {
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::Error> {
    assert!(bytes.len() == MESSAGE_LEN);
    let lhs = i32::from_be_bytes(bytes[1..5].try_into().unwrap());
    let rhs = i32::from_be_bytes(bytes[5..9].try_into().unwrap());
    match bytes[0] {
      b'I' => Ok(Message::Insert { timestamp: lhs, price: rhs }),
      b'Q' => Ok(Message::Query { mintime: lhs, maxtime: rhs }),
      _ => Err(crate::Error::UndefinedBehavior)
    }
  }
}

#[derive(Default)]
struct Session {
  entries: HashMap<i32, i32>
}

impl Session {
  fn insert(&mut self, timestamp: i32, price: i32) -> Result<(), crate::Error> {
    if self.entries.insert(timestamp, price) != None {
      Err(Error::UndefinedBehavior)
    } else {
      Ok(())
    }
  }

  fn calculate_avg(&self, mintime: i32, maxtime: i32) -> i32 {
    let range: Vec<&i32> = self.entries.iter()
      .filter(|(&timestamp, _)| timestamp >= mintime && timestamp <= maxtime )
      .map(|(_, price)| price)
      .collect();

    if range.is_empty() {
      return 0;
    }
    
    let sum: i64 = range.iter().map(|i| **i as i64).sum();
    (sum / range.len() as i64) as i32
  }

  async fn handle_client(mut self, mut stream: TcpStream) -> Result<(), crate::Error> {
    let mut buf = [0u8; MESSAGE_LEN];
    
    while let Ok(_) = stream.read_exact(&mut buf).await {
      let message = Message::from_bytes(&buf)?;
      match message {
        Message::Insert { timestamp, price } => self.insert(timestamp, price)?,
        Message::Query { mintime, maxtime } => {
          let avg = self.calculate_avg(mintime, maxtime);
          stream.write_i32(avg).await.unwrap();
        },
      };
    }
  
    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let server = TcpListener::bind(host()).await?;

  loop {
    let (client, _) = server.accept().await?;
    tokio::spawn(async move {
      let session = Session::default();
      session.handle_client(client).await
    });
  }
}

#[cfg(test)]
mod tests {
  use crate::{Message, Session};

  #[test]
  fn avg() {
    let mut s = Session::default();
    s.insert(1, 10).unwrap();
    s.insert(2, 123).unwrap();
    s.insert(3, 4).unwrap();
    s.insert(4, 3455).unwrap();
    s.insert(5, 3).unwrap();
    s.insert(6, -9).unwrap();
    assert_eq!(s.calculate_avg(1, 6), 597);
  }

  #[test]
  fn manual() {
    let bytes = [0x49, 0, 0, 0, 0xffu8, 0, 0, 0x10, 0x00];
    let msg = Message::from_bytes(&bytes).unwrap();
    assert!(matches!(msg, Message::Insert { timestamp: 255, price: 4096 }));
  }

  // #[test]
  // fn packed() {
  //   let bytes = [0xffu8, 0x10, 0x00];
  //   let msg: Message = unsafe { std::mem::transmute(bytes) };
  //   assert_eq!(msg.a, 255);
  //   assert!(msg.b.to_be() == 4096)
  // }
}
