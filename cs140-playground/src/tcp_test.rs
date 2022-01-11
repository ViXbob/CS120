use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = "10.19.73.32:18880".parse().unwrap();
    let socket = TcpSocket::new_v4()?;
    socket.bind(addr)?;
    let dst_addr = "101.32.194.18:80".parse().unwrap();
    let mut stream = socket.connect(dst_addr).await?;
    stream.try_write(b"GET / HTTP/1.1\n\n\n\n\n").unwrap();
    let mut buf = Vec::with_capacity(4096);
    loop {
        stream.readable().await.unwrap();
        match stream.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let buf : Vec<u8> = buf.iter().take(n).map(|x| *x).collect();
                println!("receive {:?}\nlen: {}", std::str::from_utf8(buf.as_ref()).unwrap_or("(invalid utf8)"), n);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                println!("error");
                break;
            }
        }
    }
    Ok(())
}