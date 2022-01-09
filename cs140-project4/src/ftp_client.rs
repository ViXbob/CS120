use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use protocol_ftp_client::{FtpTransmitter, FtpReceiver, FtpError, DataMode};
use cs140_util::tcp::athernet_tcp::AthernetTcpSocket;

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

static PORT1: u16 = 11117;
static PORT2: u16 = 11118;
static PORT3: u16 = 11119;


async fn user_get_reply(stream:&mut AthernetTcpSocket, rx_buff: &mut [u8], rx_count: &mut usize, receiver: FtpReceiver) -> FtpTransmitter {
    let mut opt_transmitter = None;
    let mut opt_receiver = Some(receiver);
    *rx_count = 0;
    while opt_receiver.is_some() {
        let data = stream.recv(PORT1).await;
        let sz = data.len();
        rx_buff[*rx_count .. *rx_count + sz].copy_from_slice(&data);
        *rx_count = *rx_count + sz;
        let ftp_receiver = opt_receiver.take().unwrap();
        match ftp_receiver.try_advance(&rx_buff[0 .. *rx_count]) {
            Ok(transmitter)   => { opt_transmitter = Some(transmitter) }
            Err(mut receiver) => {
                match receiver.take_error() {
                    Some(FtpError::NotEnoughData) => { opt_receiver = Some(receiver) }
                    Some(e)  => {
                        stream.close(PORT1).await;
                        panic!("{}", format!("Got unexpected error {}", e ))
                    }
                    _ => {
                        stream.close(PORT1).await;
                        panic!("no advance nor error?")
                    }
                };
            }
        }
    }
    opt_transmitter.unwrap()
}

fn read(buf: &mut String) {
    buf.clear();
    std::io::stdin().read_line(buf);
    let tmp = buf.trim().clone();
    *buf = String::from(tmp);
}

async fn recv(stream:&mut AthernetTcpSocket, port: u16) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    loop {
        let result = futures::executor::block_on(async {
            tokio::time::timeout(std::time::Duration::from_millis(1000),stream.recv(port)).await
        });
        if result.is_err() {
            break;
        }
        data.extend(result.unwrap());
    }
    data
}

async fn user_tcp_ftp() {
    let mut buf = String::new();
    println!("please type the address of ftp server, eg. 127.0.0.1:21");
    read(&mut buf);
    // println!("{:?}", buf);
    let addr: SocketAddrV4 = buf.parse().unwrap();
    // println!("{:?}", addr);
    // let dst_addr = Ipv4Addr::new(10, 19, 72, 77);
    let dst_addr = addr.ip().clone();
    let dst_port = addr.port();
    let mut stream = AthernetTcpSocket::new(3);
    stream.connect(dst_addr, dst_port, PORT1).await;
    let mut ftp_receiver = FtpReceiver::new();
    let mut tx_buff:[u8; 1024] = [0; 1024];
    let mut tx_count = 0;
    let mut rx_buff:[u8; 1024] = [0; 1024];
    let mut rx_count = 0;
    let mut transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;

    println!("please type your username,");
    read(&mut buf);
    // println!("{:?}", buf);

    ftp_receiver = transmitter.send_login(&mut tx_buff, &mut tx_count, buf.as_str());
    stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
    println!("login sent...");

    let mut transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
    println!("expecting password...");

    println!("please type your password,");
    read(&mut buf);
    // println!("{:?}", buf);

    ftp_receiver = transmitter.send_password(&mut tx_buff, &mut tx_count, buf.as_str());
    stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
    println!("password sent...");

    let mut transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
    println!("logged in...");
    // 10.19.95.147:21
    // 10.19.72.77:14148
    // 10.19.73.32:21

    let mut pasv_mode = 0;

    loop {
        println!("please type your command,");
        read(&mut buf);
        let mut buf_tmp = buf.clone();
        buf_tmp.push('\r');
        buf_tmp.push('\n');
        let args: Vec<&str> = buf.split(' ').collect();
        match args[0] {
            "PWD" => {
                // stream.send(Vec::from(buf_tmp.as_bytes()), PORT1).await;
                // let data = stream.recv(PORT1).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                ftp_receiver = transmitter.send_pwd_req(&mut tx_buff, &mut tx_count);
                stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());
            }
            "CWD" => {
                // stream.send(Vec::from(buf_tmp.as_bytes()), PORT1).await;
                // let data = stream.recv(PORT1).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                ftp_receiver = transmitter.send_cwd_req(&mut tx_buff, &mut tx_count, args[1]);
                stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());
            }
            "PASV" => {
                // stream.send(Vec::from(buf_tmp.as_bytes()), PORT1).await;
                // let data = stream.recv(PORT1).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                // pasv_mode = true;
                // let data = std::str::from_utf8(data.as_slice()).unwrap().trim();
                // let vec: Vec<_> = data.rsplit(' ').collect();
                // let data = *vec.first().unwrap();
                // println!("{}", data);
                // let data = data.trim_matches('.');
                // let data = data.trim_matches(')');
                // let data = data.trim_matches('(');
                // println!("{}", data);
                // let vec: Vec<_> = data.rsplit(',').collect();
                // let port: u16 = vec[0].parse::<u16>().unwrap() + vec[1].parse::<u16>().unwrap() * 256;
                ftp_receiver = transmitter.send_pasv_req(&mut tx_buff, &mut tx_count);
                stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());
                let (addr, port) = transmitter.take_endpoint().clone();
                // println!("{}", port);
                if pasv_mode == 0 {
                    stream.connect(dst_addr, port, PORT2).await;
                } else if pasv_mode == 1 {
                    stream.connect(dst_addr, port, PORT3).await;
                }
                pasv_mode = pasv_mode + 1;
            }
            "LIST" => {
                if pasv_mode == 0 {
                    println!("COMMAND ERROR: you should turn into pasv mode");
                    continue;
                }
                // stream.send(Vec::from(buf_tmp.as_bytes()), PORT1).await;
                // let data = stream.recv(PORT1).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                // let data = stream.recv(PORT2).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                ftp_receiver = transmitter.send_list_req(&mut tx_buff, &mut tx_count);
                stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());

                let data = stream.recv(PORT2).await;
                println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());

                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, transmitter.to_receiver()).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());
            }
            "RETR" => {
                if pasv_mode == 0 {
                    println!("COMMAND ERROR: you should turn into pasv mode");
                    continue;
                }
                // stream.send(Vec::from(buf_tmp.as_bytes()), PORT1).await;
                // // let data = stream.recv(PORT1).await;
                // let data = recv(&mut stream, PORT1).await;
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                // let data = recv(&mut stream, PORT1).await;
                // println!("{}", data.len());
                // println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());
                ftp_receiver = transmitter.send_get_req(&mut tx_buff, &mut tx_count, args[1]);
                stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());

                let data = stream.recv(PORT3).await;
                println!("{}", std::str::from_utf8(data.as_slice()).unwrap_or("(invalid utf8)").trim());

                transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, transmitter.to_receiver()).await;
                println!("{}", std::str::from_utf8(&rx_buff[0 .. rx_count]).unwrap_or("(invalid utf8)").trim());
            }
            "TYPE" => {
                match args[1] {
                    "ASCII" => {
                        ftp_receiver = transmitter.send_type_req(&mut tx_buff, &mut tx_count, DataMode::Text);
                        stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                        transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                        println!("{}", std::str::from_utf8(&rx_buff[.. rx_count]).unwrap_or("(invalid utf8)").trim());
                    }
                    "BINARY" => {
                        ftp_receiver = transmitter.send_type_req(&mut tx_buff, &mut tx_count, DataMode::Binary);
                        stream.send(Vec::from(&tx_buff[0 .. tx_count]), PORT1).await;
                        transmitter = user_get_reply(&mut stream, &mut rx_buff, &mut rx_count, ftp_receiver).await;
                        println!("{}", std::str::from_utf8(&rx_buff[.. rx_count]).unwrap_or("(invalid utf8)").trim());
                    }
                    _ => {
                        println!("COMMAND ERROR: you should choose type from ASCII and BINARY");
                    }
                }
            }
            "QUIT" => {
                break;
            }
            "USER" => {
                println!("COMMAND ERROR: you have already login!");
            }
            "PASS" => {
                println!("COMMAND ERROR: you have already login!");
            }
            _ => {
                println!("COMMAND ERROR: this command is not in supported list!");
            }
        }
    }

    stream.close(PORT1).await;
    stream.close(PORT2).await;
    stream.close(PORT3).await;
    println!("ftp connection terminated!");
}

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    // std_tcp_ftp();
    user_tcp_ftp().await;
    std::thread::park();
}
