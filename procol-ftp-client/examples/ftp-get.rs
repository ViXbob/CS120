extern crate url;
extern crate protocol_ftp_client;

use std::io::prelude::*;
use url::Url;
use std::net::TcpStream;
use std::env;
use std::string::String;
use std::fs::File;

use protocol_ftp_client::*;

fn get_reply(stream:&mut TcpStream, rx_buff: &mut [u8], receiver: FtpReceiver) -> FtpTransmitter {
  let mut opt_transmitter = None;
  let mut opt_receiver = Some(receiver);
  let mut total_size = 0;
  while opt_receiver.is_some() {
    let sz = stream.read(rx_buff).unwrap();
    total_size = total_size + sz;
    let ftp_receiver = opt_receiver.take().unwrap();
    match ftp_receiver.try_advance(&rx_buff[0 .. total_size]) {
      Ok(transmitter)   => { opt_transmitter = Some(transmitter) }
      Err(mut receiver) => {
        match receiver.take_error() {
          Some(FtpError::NotEnoughData) => { opt_receiver = Some(receiver) }
          Some(e)  => { panic!(format!("Got unexpected error {}", e )) }
          _ => {panic!("no advance nor error?")}
        };
      }
    }
  }
  opt_transmitter.unwrap()
}

fn main() {
  let url = env::args().nth(1).unwrap();
  println!("url: {}", url);
  let ftp_url = Url::parse(&url).unwrap();
  assert!(ftp_url.scheme() == "ftp");

  let mut username = ftp_url.username();
  if username == "" { username = "anonymous" };

  let password = match ftp_url.password() {
    Some(value) => value,
    None        => "unspecified",
  };

  assert!(ftp_url.path() != "");

  let host = ftp_url.host().unwrap();
  let port:u16 = ftp_url.port().or(Some(21)).unwrap();
  let filename = ftp_url.path_segments().unwrap().last().unwrap();
  let remote_path = ftp_url.path_segments().unwrap()
    .take_while(|part| part.to_string() != filename.to_string())
    .fold(String::new(), |mut acc, part| { acc.push_str("/"); acc.push_str(part); acc } );

  println!("start dowloading {} at {}...", filename, remote_path);

  let mut tx_buff:[u8; 1024] = [0; 1024];
  let mut tx_count = 0;
  let mut rx_buff:[u8; 1024] = [0; 1024];

  let host_port = format!("{}:{}", host, port);
  let mut stream = TcpStream::connect(host_port.as_str()).unwrap();
  let mut ftp_receiver = FtpReceiver::new();

  let mut transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("connected to {}:{}", host, port);

  ftp_receiver = transmitter.send_login(&mut tx_buff, &mut tx_count, username);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  println!("login sent...");

  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("expecting password...");

  ftp_receiver = transmitter.send_password(&mut tx_buff, &mut tx_count, password);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  println!("password sent...");

  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("logged in...");

  ftp_receiver = transmitter.send_system_req(&mut tx_buff, &mut tx_count);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  {
    let (system, subtype) = transmitter.get_system().clone();
    println!("remote system {} / {}", system, subtype);
  }

  ftp_receiver = transmitter.send_cwd_req(&mut tx_buff, &mut tx_count, &remote_path);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("cwd to {}", remote_path);

  ftp_receiver = transmitter.send_pwd_req(&mut tx_buff, &mut tx_count);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("changed remote directory is {}", transmitter.get_wd());

  ftp_receiver = transmitter.send_type_req(&mut tx_buff, &mut tx_count, DataMode::Binary);
  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("switched to binary mode");

  let mut data_stream = {
    ftp_receiver = transmitter.send_pasv_req(&mut tx_buff, &mut tx_count);
    let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
    transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
    let (addr, port) = transmitter.take_endpoint().clone();
    println!("confirmed passive connection on {}:{}", addr, port);
    TcpStream::connect((addr, port)).unwrap()
  };
  println!("passive connection opened");

  ftp_receiver = transmitter.send_get_req(&mut tx_buff, &mut tx_count, ftp_url.path());

  let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
  transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
  println!("starting downloading file {}", filename);

  let mut local_file = File::create(filename).unwrap();
  let mut eof = false;
  let mut data_in =  [0; 40960];
  while !eof {
    let count = data_stream.read(&mut data_in).unwrap();
    // println!("got {} bytes", count);
    eof = count == 0;
    if !eof { let _ = local_file.write(&data_in[0 .. count]).unwrap(); };
  }
  local_file.flush().unwrap();
  println!("");

  println!("got file {}", filename);
  let _ = get_reply(&mut stream, &mut rx_buff, transmitter.to_receiver());
  println!("Success ... ");

}
