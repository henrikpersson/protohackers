use std::{net::UdpSocket, io, collections::HashMap};

const PACKET_LEN: usize = 1000;

#[derive(PartialEq, Eq, Debug)]
enum Request<'a> {
  Insert(Option<&'a str>, &'a str),
  Get(&'a str)
}

impl<'a> Request<'a> {
  pub fn from_buf(buf: &'a [u8]) -> Self {
    let eq_sign = buf.iter().enumerate().find(|(_, &b)| b == b'=');
    match eq_sign {
      Some((0, _)) => Request::Insert(None, std::str::from_utf8(&buf[1..]).unwrap()),
      Some((len, _)) => Request::Insert(Some(std::str::from_utf8(&buf[..len]).unwrap()), std::str::from_utf8(&buf[len+1..]).unwrap()),
      None => Request::Get(std::str::from_utf8(buf).unwrap())
    }
  }
}

fn main() -> Result<(), io::Error> {
  let host = std::env::var("HOST").expect("HOST ENV");
  let socket = UdpSocket::bind(host)?;

  let version = "version=bleh's Key-Value Store 2.0".as_bytes();
  let mut store: HashMap<String, String> = HashMap::with_capacity(1000);

  let mut buf = [0u8; PACKET_LEN];
  while let Ok((read, client)) = socket.recv_from(&mut buf) {
    let packet = &buf[..read];
    let req = Request::from_buf(packet);
    println!("{:?}", req);
    match req {
      Request::Insert(None, _) => (),
      Request::Insert(Some(key), value) => {
        store.insert(key.to_string(), value.to_string());
      }
      Request::Get("version") => {
        socket.send_to(version, client).unwrap();
      }
      Request::Get(key) => {
        let value = store.get(key).map(|s|s.as_str()).unwrap_or("");
        let response = format!("{}={}", key, value);
        socket.send_to(response.as_bytes(), client).unwrap();
      }
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::Request;

  #[test]
  fn test() {
    assert_eq!(Request::from_buf(&[b'=', 0x41, 0x42]), Request::Insert(None, "AB"));
    assert_eq!(Request::from_buf(&[0x43, b'=', 0x41, 0x42]), Request::Insert(Some("C"), "AB"));
    assert_eq!(Request::from_buf(&[0x43, b'=', b'=', 0x41, 0x42]), Request::Insert(Some("C"), "=AB"));
    assert_eq!(Request::from_buf(&[0x43, b'=']), Request::Insert(Some("C"), ""));
    assert_eq!(Request::from_buf(&[0x43, 0x41, 0x42]), Request::Get("CAB"));
  }
}