use regex::Regex;
use tokio::{net::{TcpListener, TcpStream}, io::{ReadHalf, WriteHalf, BufReader, AsyncBufReadExt, AsyncWriteExt}};

const UPSTREAM_ADDR: &str = "chat.protohackers.com:16963";
const TONY_ADDR: &str = "${1}7YWHMfk9JZe0LM0g1ZauHuiSxhI${2}";
const PATTERN: &str = r"( ?)7[?:A-Za-z0-9]{25,34}($| |\n)";

fn proxy(rx: ReadHalf<TcpStream>, mut tx: WriteHalf<TcpStream>) {
  tokio::spawn(async move {
    let re = Regex::new(PATTERN).unwrap();
    let mut buf = Vec::with_capacity(1024);
    let mut reader = BufReader::new(rx);
    while let Ok(nread) = reader.read_until(b'\n', &mut buf).await {
      if nread == 0 {
        break;
      }
      let msg = String::from_utf8(buf.to_vec()).unwrap();
      let steal = re.replace_all(msg.as_str(), TONY_ADDR);
      tx.write_all(steal.as_bytes()).await.unwrap();
      buf.clear();
    }
    tx.shutdown().await.unwrap();
  });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let host = std::env::var("HOST")?;
  
  println!("l on {}", host);
  let server = TcpListener::bind(host).await?;
  
  loop {
    let (downstream, _) = server.accept().await?;
    let upstream = TcpStream::connect(UPSTREAM_ADDR).await.unwrap();
    let (up_rx, up_tx) = tokio::io::split(upstream);
    let (down_rx, down_tx) = tokio::io::split(downstream);
    proxy(up_rx, down_tx);
    proxy(down_rx, up_tx);
  }
}