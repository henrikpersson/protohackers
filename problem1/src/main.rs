use std::{net::{TcpListener, TcpStream}, io::{Write, BufReader, BufWriter, BufRead}};
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Deserialize, PartialEq)]
enum Error {
  #[error("Floating point")]
  FloatingPoint,
  #[error("Invalid JSON")]
  InvalidJSON(String)
}

impl std::convert::From<serde_json::Error> for Error {
  fn from(err: serde_json::Error) -> Self {
    return err.to_string()
      .find("invalid type: floating point ")
      .map_or(Error::InvalidJSON(err.to_string()), |_| Error::FloatingPoint);
  }
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
struct Request<'a> {
  method: &'a str,
  number: isize
}

#[derive(Serialize, Debug)]
struct Response<'a> {
  method: &'a str,
  prime: bool
}

impl Response<'_> {
  pub fn ok(prime: bool) -> Self {
    Self { method: "isPrime", prime } 
  }

  pub fn malformed() -> Self {
    Self { method: "malformed", prime: false } 
  }

  pub fn is_malformed(&self) -> bool {
    return self.method == "malformed"
  }
}

struct Session;

impl Session {
  pub fn new() -> Self {
    Self {}
  }

  fn parse_request<'a>(&'a self, json: &'a str) -> Result<Request, Error> {
    serde_json::from_str(json).map_err(|e| e.into())
  }
  
  fn handle_request(&self, json: &str) -> Response {
    let req = self.parse_request(json);
    match req {
      Ok(Request { method: "isPrime", number}) => Response::ok(self.is_prime(number)),
      Ok(Request { method: _, number: _ }) => Response::malformed(),
      Err(Error::FloatingPoint) => Response::ok(false),
      Err(Error::InvalidJSON(_)) => Response::malformed()
    }
  }

  fn is_prime(&self, number: isize) -> bool {
    if number <= 1 {
      return false
    }
    
    let sqrt = ((number as f32).sqrt()) as isize;
    for i in 2..=sqrt {
      if number % i == 0 {
        return false;
      }
    }
  
    return true;
  }
  
  pub fn start(self, stream: &TcpStream) {
    println!("client connected!");
    
    let mut buf: Vec<u8> = Vec::with_capacity(200);
    let mut read = BufReader::new(stream);
    let mut write = BufWriter::new(stream);
  
    loop {
      read.read_until(b'\n', &mut buf).unwrap();
      let json = String::from_utf8(buf.clone()).unwrap();
      let json = json.trim();
      buf.clear();
  
      let resp = self.handle_request(&json);
      let json_resp = serde_json::to_string(&resp).unwrap();
      println!("request: {}, response: {}", json, json_resp);
      write.write_fmt(format_args!("{}\n", json_resp)).unwrap();
      write.flush().unwrap();
      if resp.is_malformed() {
        break;
      }
    }
  }
}

struct Server;

impl Server {
  pub fn new() -> Self {
    Self { }
  }

  pub fn listen(self) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("206.189.10.100:4444")?;
    // let server = TcpListener::bind("127.0.0.1:4444")?;

    for client in listener.incoming() {
      std::thread::spawn(move || { 
        let stream = client.unwrap();
        let session = Session::new();
        session.start(&stream);
      });
    }

    Ok(())
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let server = Server::new();
  server.listen()?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::{Request, Error, Session};

  #[test]
  fn ok_parse() {
    let json = "{\"method\":\"isPrime\",\"number\":123}";
    let expected = Request { method: "isPrime", number: 123 };
    assert_eq!(Session::new().parse_request(json).unwrap(), expected)
  }

  #[test]
  fn floating_point() {
    let json = "{\"method\":\"isPrime\",\"number\":123.1}";
    assert_eq!(Session::new().parse_request(json).unwrap_err(), Error::FloatingPoint)
  }

  #[test]
  fn invalid_json() {
    let json = "{\"method\":isPrime\",\"number\":123.1}";
    assert!(matches!(Session::new().parse_request(json).unwrap_err(), Error::InvalidJSON{..}))
  }

  #[test]
  fn signed() {
    let json = "{\"method\":\"isPrime\",\"number\":-4}";
    let expected = Request { method: "isPrime", number: -4 };
    assert_eq!(Session::new().parse_request(json).unwrap(), expected)
  }

  #[test]
  fn primes() {
    let expected = [2, 3, 5, 7, 11, 13, 17, 19, 23];
    let mut primes: Vec<isize> = vec![];
    let mut curr = 0;
    while primes.len() != expected.len() {
      if Session::new().is_prime(curr) {
        primes.push(curr)
      }
      curr += 1;
    }
    assert_eq!(primes, expected);
  }
}