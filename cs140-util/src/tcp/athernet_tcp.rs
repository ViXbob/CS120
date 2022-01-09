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
use smoltcp::iface::SocketHandle;
use tokio::{
    sync::mpsc::{channel, Sender, Receiver},
    runtime::Handle,
};

pub enum TcpSocketCommand {
    Send(Vec<u8>),
    Close,
}

pub struct AthernetTcpSocket {
    command_send: Vec<(Option<u16>, Sender<TcpSocketCommand>)>,
    package_recv: Vec<(Option<u16>, Receiver<Vec<u8>>)>,
    quit_signal_recv: Vec<(Option<u16>, Receiver<()>)>,
    connect_command: Sender<((u16, (Ipv4Addr, u16)), usize)>,
    index: usize,
    tcp_socket_count: usize,
}

impl AthernetTcpSocket {
    pub fn new(tcp_socket_count: usize) -> Self {
        let mut command_send: Vec<(Option<u16>, Sender<TcpSocketCommand>)> = Vec::new();
        let mut command_recv: Vec<(Option<u16>, Receiver<TcpSocketCommand>)> = Vec::new();
        let mut package_send: Vec<(Option<u16>, Sender<Vec<u8>>)> = Vec::new();
        let mut package_recv: Vec<(Option<u16>, Receiver<Vec<u8>>)> = Vec::new();
        let mut quit_signal_recv: Vec<(Option<u16>, Receiver<()>)> = Vec::new();
        let mut quit_signal_send: Vec<(Option<u16>, Sender<()>)> = Vec::new();
        let mut tcp_active: Vec<bool> = Vec::new();
        let (connect_send, mut connect_recv) = channel::<((u16, (Ipv4Addr, u16)), usize)>(1024);
        let index_now = 0;
        for _ in 0..tcp_socket_count {
            let (command_send_, mut command_recv_) = channel::<TcpSocketCommand>(1024);
            let (package_send_, package_recv_) = channel::<Vec<u8>>(1024);
            let (quit_signal_send_, quit_signal_recv_) = channel(1024);
            command_send.push((None, command_send_));
            command_recv.push((None, command_recv_));
            package_send.push((None, package_send_));
            package_recv.push((None, package_recv_));
            quit_signal_recv.push((None, quit_signal_recv_));
            quit_signal_send.push((None, quit_signal_send_));
            tcp_active.push(false);
        }
        tokio::task::spawn_blocking(move || {
            let mut tcp_handle: Vec<(Option<u16>, SocketHandle, VecDeque<TcpSocketCommand>)> = Vec::new();
            let mtu: usize = 256;
            let tcp_client = Mutex::new(TCPClient::new(mtu));
            for _ in 0..tcp_socket_count {
                let handle = tcp_client.lock().unwrap().new_socket();
                let q1: VecDeque<TcpSocketCommand> = VecDeque::new();
                tcp_handle.push((None, handle, q1));
            }
            // {
            //     let mut guard = tcp_client.lock().unwrap();
            //     guard.connect(dst_addr, dst_port, local_port);
            // }
            // let mut tcp_active = false;
            loop {
                let mut guard = tcp_client.lock().unwrap();
                let timestamp = Instant::now();
                let result = connect_recv.try_recv();
                if !result.is_err() {
                    let ((src_port, (dst_addr, dst_port)), index) = result.unwrap();
                    tcp_handle[index].0 = Some(src_port);
                    guard.connect(dst_addr, dst_port, src_port, tcp_handle[index].1);
                }
                match guard.iface.poll(timestamp) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("poll error: {}", e);
                    }
                }
                for index in 0..tcp_socket_count {
                    let handle: SocketHandle = tcp_handle[index].1.clone();
                    if tcp_handle[index].0.is_none() { continue; }
                    let command = command_recv[index].1.try_recv();
                    if !command.is_err() {
                        tcp_handle[index].2.push_back(command.unwrap());
                    }
                    let (need_disconnect, data) = guard.use_socket(|socket| {
                        let need_disconnect = if socket.is_active() && !tcp_active[index] {
                            // debug!("connected");
                            false
                        } else if !socket.is_active() && tcp_active[index] {
                            // debug!("disconnected");
                            return (true, None);
                        } else {
                            false
                        };
                        tcp_active[index] = socket.is_active();

                        while !tcp_handle[index].2.is_empty() {
                            let command = tcp_handle[index].2.front().unwrap();
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
                            tcp_handle[index].2.pop_front();
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
                    }, handle);
                    if need_disconnect {
                        quit_signal_send[index].1.blocking_send(());
                    }
                    if let Some(data) = data {
                        if !data.is_empty() {
                            package_send[index].1.blocking_send(data);
                        }
                    }
                }
            }
        });

        AthernetTcpSocket {
            command_send,
            package_recv,
            quit_signal_recv,
            connect_command: connect_send,
            index: index_now,
            tcp_socket_count
        }
    }
    pub async fn send(&self, data: Vec<u8>, src_port: u16) {
        for (port, command_send_) in &self.command_send {
            if port.is_none() { continue; }
            if port.unwrap() != src_port { continue; }
            command_send_.send(TcpSocketCommand::Send(data.clone())).await;
        }
    }
    pub async fn recv(&mut self, src_port: u16) -> Vec<u8> {
        let mut data: Option<Vec<u8>> = None;
        loop {
            for index in 0..self.tcp_socket_count {
                if self.package_recv[index].0.is_none() { continue; }
                if self.package_recv[index].0.unwrap() != src_port { continue; }
                data = Some(self.package_recv[index].1.recv().await.unwrap());
            }
            if !data.is_none() { break; }
        }
        data.unwrap()
    }
    pub async fn close(&mut self, src_port: u16) {
        for (index, (port, command_send_)) in self.command_send.iter().enumerate() {
            if port.is_none() { continue; }
            if port.unwrap() != src_port { continue; }
            command_send_.send(TcpSocketCommand::Close).await;
            self.quit_signal_recv[index].1.recv().await;
        }
    }
    pub async fn connect(&mut self, dst_addr: Ipv4Addr, dst_port: u16, src_port: u16) {
        if self.index >= self.tcp_socket_count {
            return;
        }
        for (port, _) in &self.command_send {
            if port.is_none() { continue; }
            if port.unwrap() == src_port { return; }
        }
        self.command_send[self.index].0 = Some(src_port);
        self.package_recv[self.index].0 = Some(src_port);
        self.connect_command.send(((src_port, (dst_addr, dst_port)), self.index)).await;
        self.index += 1;
    }
}