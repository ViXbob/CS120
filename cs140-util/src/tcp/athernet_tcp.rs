use std::{
    net::Ipv4Addr,
    collections::VecDeque,
};
use crate::tcp::tcp_stack::TCPClient;
use smoltcp::{
    time::Instant,
    socket::TcpSocket,
    wire::Ipv4Packet,
};
use log::{trace, debug, info, warn};
use tokio::{
    sync::mpsc::{channel, Sender, Receiver},
    runtime::Handle,
};

pub enum TcpSocketCommand {
    Send(Vec<u8>),
    Close,
}

pub struct AthernetTcpSocket {
    command_send: Sender<TcpSocketCommand>,
    package_recv: Receiver<Vec<u8>>,
}

impl AthernetTcpSocket {
    pub fn new(dst_addr: Ipv4Addr, dst_port: u16, local_port: u16) -> Self {
        let (command_send, mut command_recv) = channel::<TcpSocketCommand>(1024);
        let (package_send, package_recv) = channel::<Vec<u8>>(1024);
        std::thread::spawn(move || {
            let mut q: VecDeque<TcpSocketCommand> = VecDeque::new();
            let mtu: usize =  256;
            let mut tcp_client = TCPClient::new(mtu);
            tcp_client.connect(dst_addr, dst_port, local_port);
            let mut tcp_active = false;
            loop {
                let timestamp = Instant::now();
                match tcp_client.iface.poll(timestamp) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("poll error: {}", e);
                    }
                }
                let socket = tcp_client.iface.get_socket::<TcpSocket>(tcp_client.tcp_handle);
                if socket.is_active() && !tcp_active {
                    debug!("connected");
                } else if !socket.is_active() && tcp_active {
                    debug!("disconnected");
                    break;
                }
                tcp_active = socket.is_active();
                let command = command_recv.try_recv();
                if !command.is_err() {
                    q.push_back(command.unwrap());
                }
                while !q.is_empty() {
                    let command = q.front().unwrap();
                    let mut good: bool = false;
                    match command {
                        TcpSocketCommand::Send(data) => {
                            if socket.can_send() {
                                let result = socket.send_slice(data.as_slice());
                                if result.is_err() {
                                    warn!("athernet tcp failed to send a package!");
                                } else {
                                    good = true;
                                }
                            }
                        }
                        TcpSocketCommand::Close => {
                            socket.close();
                            good = true
                        }
                    }
                    if !good { break; }
                    q.pop_front();
                }
                if socket.may_recv() {
                    let data = socket
                        .recv(|data| {
                            let mut data = data.to_owned();
                            if !data.is_empty() {
                                debug!(
                                    "recv data: {:?}",
                                    std::str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                                );
                                // data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                                // data.reverse();
                                // data.extend(b"\n");
                            }
                            (data.len(), data)
                        })
                        .unwrap();
                    let handle = Handle::current();
                    handle.enter();
                    futures::executor::block_on(async {
                        package_send.send(data).await;
                    });
                }
            }
        });

        AthernetTcpSocket {
            command_send,
            package_recv,
        }
    }
    pub async fn send(&self, data: Vec<u8>) {
        self.command_send.send(TcpSocketCommand::Send(data)).await;
    }
    pub async fn recv(&mut self) -> Vec<u8> {
        self.package_recv.recv().await.unwrap()
    }
}