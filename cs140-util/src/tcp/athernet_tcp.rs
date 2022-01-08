use std::{
    net::Ipv4Addr,
    collections::VecDeque,
};
use std::sync::Mutex;
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
    quit_signal_recv: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl AthernetTcpSocket {
    pub fn new(dst_addr: Ipv4Addr, dst_port: u16, local_port: u16) -> Self {
        let (command_send, mut command_recv) = channel::<TcpSocketCommand>(1024);
        let (package_send, package_recv) = channel::<Vec<u8>>(1024);
        let (quit_signal_send, quit_signal_recv) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let mut q: VecDeque<TcpSocketCommand> = VecDeque::new();
            let mtu: usize = 256;
            let tcp_client = Mutex::new(TCPClient::new(mtu));
            {
                let mut guard = tcp_client.lock().unwrap();
                guard.connect(dst_addr, dst_port, local_port);
            }
            let mut tcp_active = false;
            loop {
                let (need_disconnect, data) = {
                    let mut guard = tcp_client.lock().unwrap();
                    let timestamp = Instant::now();
                    match guard.iface.poll(timestamp) {
                            Ok(_) => {}
                        Err(e) => {
                            debug!("poll error: {}", e);
                        }
                    }
                    guard.use_socket(|socket| {
                        let need_disconnect = if socket.is_active() && !tcp_active {
                            debug!("connected");
                            false
                        } else if !socket.is_active() && tcp_active {
                            debug!("disconnected");
                            return (true, None);
                        } else {
                            false
                        };
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
                        let data = if socket.may_recv() {
                            Some(socket
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
                                .unwrap())
                        } else {
                            None
                        };
                        (need_disconnect, data)
                    })
                };
                if need_disconnect {
                    break;
                }
                if let Some(data) = data {
                    if !data.is_empty() {
                        package_send.blocking_send(data);
                    }
                }
            }
            quit_signal_send.send(()).unwrap();
        });

        AthernetTcpSocket {
            command_send,
            package_recv,
            quit_signal_recv:Some(quit_signal_recv),
        }
    }
    pub async fn send(&self, data: Vec<u8>) {
        self.command_send.send(TcpSocketCommand::Send(data)).await;
    }
    pub async fn recv(&mut self) -> Vec<u8> {
        self.package_recv.recv().await.unwrap()
    }
    pub async fn close(&mut self) {
        self.command_send.send(TcpSocketCommand::Close).await;
        let recv = std::mem::replace(&mut self.quit_signal_recv, None).unwrap();
        recv.await;
    }
}