use crate::packet::MyPacket;
use std::io::{self, ErrorKind};
use std::net::UdpSocket;

pub enum RecvRes {
    Timeout,
    Break,
    Code(MyPacket),
}

impl RecvRes {
    pub fn try_recv(sock: &UdpSocket, try_times: u32) -> io::Result<RecvRes> {
        let mut buf = [0u8; 1500];
        let recv_res = sock.recv_from(&mut buf);
        if let Err(e) = recv_res {
            if e.kind() == ErrorKind::WouldBlock {
                if try_times < 5 {
                    Ok(Self::Timeout)
                } else {
                    Ok(Self::Break)
                }
            } else {
                Err(e)
            }
        } else {
            let (amt, src) = recv_res.unwrap();
            Ok(Self::Code(MyPacket::from(&buf[..amt], src)))
        }
    }
}

pub fn try_send(sock: &UdpSocket, packet: MyPacket) -> io::Result<()> {
    let src = packet.src;
    sock.send_to(packet.to_vec().as_slice(), src)?;
    Ok(())
}
