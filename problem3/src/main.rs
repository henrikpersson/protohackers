use std::{collections::HashMap};

use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, BufReader, AsyncBufReadExt, ReadHalf, WriteHalf}, sync::{mpsc::{self, Receiver}}};

#[derive(Debug)]
enum Event {
  UserEnter(usize, User),
  NewMessage(usize, String),
  UserLeave(usize)
}

#[derive(Debug)]
struct User {
  name: String,
  socket: WriteHalf<TcpStream>,
}

#[derive(Default)]
struct Room {
  users: HashMap<usize, User>
}

impl Room {
  pub async fn open(mut self, mut rx: Receiver<Event>) {
    while let Some(message) = rx.recv().await {
      // dbg!(&message);
      match message {
        Event::UserEnter(id, mut user) => {
          let msg = format!("* {} has entered the room\n", user.name);
          self.broadcast(id, msg).await;

          let here: Vec<&str> = self.users.values().map(|u| u.name.as_str()).collect();
          let here: Vec<&str> = here.into_iter().filter(|&n| n != user.name).collect();
          let hello = format!("* The room contains: {}\n", here.join(", "));
          user.socket.write(hello.as_bytes()).await.unwrap();

          self.users.insert(id, user);
        }
        Event::NewMessage(from_id, msg) => {
          if msg.len() == 0 {
            return;
          }
          let user = self.users.get(&from_id).unwrap();
          let msg = format!("[{}] {}\n", user.name, msg);
          self.broadcast(from_id, msg).await;
        },
        Event::UserLeave(id) => {
          let user = self.users.get(&id).unwrap();
          let msg = format!("* {} has left the room\n", user.name);
          self.broadcast(id, msg).await;
          self.users.remove(&id);
        },
      };
    }
  }

  async fn broadcast(&mut self, from_id: usize, msg: String) {
    let users: Vec<&mut User> = self.users.iter_mut()
      .filter(|(&id, _)| id != from_id)
      .map(|(_, user)| user)
      .collect();

    for user in users {
      user.socket.write(msg.as_bytes()).await;
    }
  }
}

#[derive(Default)]
struct Server;

impl Server {
  pub async fn serve(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::var("HOST")?;
    println!("l on {}", host);
    let server = TcpListener::bind(host).await?;

    let (tx, rx) = mpsc::channel::<Event>(32);
    tokio::spawn(async move {
      let room = Room::default();
      room.open(rx).await;
    });

    let mut id = 0;
    loop {
      let (socket, _) = server.accept().await?;
      let tx = tx.clone();
      tokio::spawn(async move {
        let id = id;
        let (read, write) = tokio::io::split(socket);
        let mut read = BufReader::new(read);
        if let Ok(user) = Self::accept_user(&mut read, write).await {
          tx.send(Event::UserEnter(id, user)).await.unwrap();

          let mut buf: Vec<u8> = vec![];
          while let Ok(r) = read.read_until(b'\n', &mut buf).await {
            if r == 0 {
              break;
            }
            let msg = String::from_utf8(buf.clone()).unwrap();
            if msg.len() != 0 {
              tx.send(Event::NewMessage(id, msg.trim().to_string())).await.unwrap();
            }
            buf.clear();
          }

          tx.send(Event::UserLeave(id)).await.unwrap();
        }
      });

      id += 1;
    }
  }

  async fn accept_user(rx: &mut BufReader<ReadHalf<TcpStream>>, mut tx: WriteHalf<TcpStream>) -> Result<User, Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = vec![];
    tx.write("Welcome to budgetchat! What shall I call you?\n".as_bytes()).await?;
    rx.read_until(b'\n', &mut buf).await?;

    let name = String::from_utf8(buf)?;
    let name = name.trim().to_string();
    if name.len() < 1 {
      return Err("too short".into());
    }

    if !name.chars().all(|c| c.is_alphanumeric()) {
      return Err("invalid".into());
    }

    Ok(User { name, socket: tx })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut server = Server::default();
  server.serve().await?;
  Ok(())
}