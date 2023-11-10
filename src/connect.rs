use crate::config;
use crate::packet;
use rand::prelude::*;
use std::error;
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub struct Connection {
    pub remote_addr: SocketAddr,
    pub local: UdpSocket,
    pub timeout: u32,
    pub seq: u8,
    pub est: bool,
}

impl Connection {
    pub fn new(config: &config::Config) -> Connection {
        let (lport, rport) = if config.is_server {
            (6500, 6501)
        } else {
            (6501, 6500)
        };
        let remote_addr = format!("127.0.0.1:{}", rport)
            .parse::<SocketAddr>()
            .unwrap();
        let local_addr = format!("127.0.0.1:{}", lport)
            .parse::<SocketAddr>()
            .unwrap();
        let local = UdpSocket::bind(local_addr).unwrap();
        local.set_read_timeout(Some(config.single_timeout)).unwrap();
        local.set_write_timeout(Some(config.single_timeout)).unwrap();
        let timeout = 0;
        let seq = 0;
        Connection {
            remote_addr,
            local,
            timeout,
            seq,
            est: false,
        }
    }
}

#[derive(Debug)]
pub enum RecvRes {
    Timeout,
    Break,
    Get(Vec<packet::MyPacket>),
}

pub fn try_recv_lossy(
    sock: &UdpSocket,
    try_times: u32,
    cfg: &config::Config,
) -> Result<RecvRes, Box<dyn error::Error>> {
    match try_recv(sock)? {
        RecvRes::Timeout => Ok(if try_times == cfg.max_timeout - 1 {
            RecvRes::Break
        } else {
            RecvRes::Timeout
        }),
        RecvRes::Break => panic!("Inner function error"),
        RecvRes::Get(pkts) => {
            let mut res = Vec::new();
            let mut rng = thread_rng();
            for pkt in pkts {
                if rng.gen::<f64>() < cfg.receive_rate {
                    res.push(pkt);
                }
            }
            if res.len() == 0 {
                Ok(RecvRes::Timeout)
            } else {
                Ok(RecvRes::Get(res))
            }
        }
    }
}

fn try_recv(sock: &UdpSocket) -> Result<RecvRes, Box<dyn error::Error>> {
    let mut buffer = [0u8; 65535];
    let res = sock.recv(&mut buffer);
    if let Err(e) = res {
        if e.kind() == io::ErrorKind::WouldBlock {
            Ok(RecvRes::Timeout)
        } else {
            Err(Box::new(e))
        }
    } else {
        let size = res.unwrap();
        if size == 0 {
            return Ok(RecvRes::Timeout);
        }
        Ok(RecvRes::Get(packet::MyPacket::from(&buffer[..size])?))
    }
}

pub fn try_send_lossy(
    connect: &Connection,
    packets: Vec<&packet::MyPacket>,
    send_rate: f64,
) -> io::Result<()> {
    let mut send_packets = Vec::new();
    let mut rng = thread_rng();
    for packet in packets {
        // decide to send by random
        if rng.gen::<f64>() < send_rate {
            send_packets.push(packet);
        }
    }
    try_send(&connect.local, &connect.remote_addr, send_packets)?;
    Ok(())
}

pub fn try_send(
    sock: &UdpSocket,
    addr: &SocketAddr,
    packets: Vec<&packet::MyPacket>,
) -> io::Result<()> {
    let mut bytes = Vec::new();
    for p in packets {
        bytes.append(&mut p.to_vec());
    }
    sock.send_to(bytes.as_slice(), addr)?;
    Ok(())
}

#[test]
fn test_send() {
    // test send and recv
    use std::thread;
    let handle = thread::spawn(|| {
        let sock = UdpSocket::bind("127.0.0.1:6501".parse::<SocketAddr>().unwrap()).unwrap();
        let result = try_recv(&sock).unwrap();
        println!("{:?}", result);
    });
    let sock = UdpSocket::bind("127.0.0.1:6500".parse::<SocketAddr>().unwrap()).unwrap();
    let pkts = vec![
        packet::MyPacket::with_code(200),
        packet::MyPacket::with_data(vec![1, 3, 4], 12),
    ];
    try_send(
        &sock,
        &"127.0.0.1:6501".parse::<SocketAddr>().unwrap(),
        pkts.iter().collect(),
    )
    .unwrap();
    handle.join().unwrap();
}
