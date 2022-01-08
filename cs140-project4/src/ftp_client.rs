use std::io::{Read, Write};
use async_ftp::FtpStream;
use std::net::TcpStream;
use protocol_ftp_client::{FtpTransmitter, FtpReceiver, FtpError};

fn get_reply(stream:&mut TcpStream, rx_buff: &mut [u8], receiver: FtpReceiver) -> FtpTransmitter {
    let mut opt_transmitter = None;
    let mut opt_receiver = Some(receiver);
    let mut total_size = 0;
    while opt_receiver.is_some() {
        let sz = stream.read(rx_buff).unwrap();
        println!("size: {}", sz);
        total_size = total_size + sz;
        let ftp_receiver = opt_receiver.take().unwrap();
        match ftp_receiver.try_advance(&rx_buff[0 .. total_size]) {
            Ok(transmitter)   => { opt_transmitter = Some(transmitter) }
            Err(mut receiver) => {
                match receiver.take_error() {
                    Some(FtpError::NotEnoughData) => { opt_receiver = Some(receiver) }
                    Some(e)  => { panic!("{}", format!("Got unexpected error {}", e )) }
                    _ => {panic!("no advance nor error?")}
                };
            }
        }
    }
    opt_transmitter.unwrap()
}

fn std_tcp_ftp() {
    let mut stream = TcpStream::connect("10.19.72.77:14148").unwrap();
    let mut ftp_receiver = FtpReceiver::new();
    let mut tx_buff:[u8; 1024] = [0; 1024];
    let mut tx_count = 0;
    let mut rx_buff:[u8; 1024] = [0; 1024];
    let mut transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);

    ftp_receiver = transmitter.send_login(&mut tx_buff, &mut tx_count, "anonymous");
    let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
    println!("login sent...");

    transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);
    println!("expecting password...");

    ftp_receiver = transmitter.send_password(&mut tx_buff, &mut tx_count, "123456");
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

    ftp_receiver = transmitter.send_pasv_req(&mut tx_buff, &mut tx_count);
    let _ = stream.write_all(&tx_buff[0 .. tx_count]).unwrap();
    transmitter = get_reply(&mut stream, &mut rx_buff, ftp_receiver);

}

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    std_tcp_ftp();
}
