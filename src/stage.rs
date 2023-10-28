use crate::packet::MyPacket;
use crate::recv_res::{self, RecvRes};
use std::collections::VecDeque;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};
use std::process;
use std::sync::mpsc;
use std::time;

pub enum SendStatus {
    Ready,
    OnFly(time::Instant),
    Acked,
}

pub fn stage0(
    sock: &UdpSocket,
    queue: &mpsc::Receiver<String>,
    try_times: &mut u32,
) -> Result<u32, Box<dyn Error>> {
    // read stdin (TODO later)
    if let Ok(s) = queue.try_recv() {
        match s.as_str() {
            "quit" => process::exit(0),
            "testgbn" => {
                let src: SocketAddr = "127.0.0.1:6500".parse().unwrap();
                recv_res::try_send(sock, MyPacket::with_code(205, &src))?;
                return Ok(1);
            }
            _ => println!("unrecognized command"),
        }
    }
    if let RecvRes::Code(packet) = RecvRes::try_recv(sock, *try_times)? {
        if packet.code == 205 {
            recv_res::try_send(sock, MyPacket::with_code(205, &packet.src))?;
            *try_times = 0;
            return Ok(1);
        }
    }
    Ok(0)
}

pub fn stage1(
    sock: &UdpSocket,
    try_times: &mut u32,
    resend_packet: &MyPacket,
) -> Result<u32, Box<dyn Error>> {
    let src = resend_packet.src;
    match RecvRes::try_recv(sock, *try_times)? {
        RecvRes::Code(packet) => {
            if packet.code == 200 {
                recv_res::try_send(sock, MyPacket::with_code(200, &src))?;
                *try_times = 0;
                Ok(3)
            } else {
                recv_res::try_send(sock, MyPacket::with_code(204, &src))?;
                Ok(1)
            }
        }
        RecvRes::Timeout => {
            recv_res::try_send(sock, MyPacket::with_code(205, &src))?;
            *try_times += 1;
            Ok(1)
        }
        RecvRes::Break => Ok(0),
    }
}

pub fn stage2(
    sock: &UdpSocket,
    try_times: &mut u32,
    resend_packets: &mut VecDeque<(MyPacket, SendStatus)>,
) -> Result<u32, Box<dyn Error>> {
    if resend_packets.is_empty() {
        return Ok(0);
    }
    // check timeout first to avoid repeat send
    let mut timeout_flag = false;
    for (_, status) in resend_packets.iter_mut() {
        if let SendStatus::OnFly(now) = status {
            if timeout_flag {
                *status = SendStatus::Ready;
            } else if now.elapsed() > time::Duration::new(3, 0) {
                timeout_flag = true;
            }
        }
    }

    for (packet, status) in resend_packets.iter_mut() {
        if let SendStatus::Ready = status {
            *status = SendStatus::OnFly(time::Instant::now());
            recv_res::try_send(sock, packet.clone())?;
        }
    }

    match RecvRes::try_recv(sock, *try_times)? {
        RecvRes::Code(recv_packet) => {
            for (packet, status) in resend_packets.iter_mut() {
                if let SendStatus::OnFly(_) = status {
                    if recv_packet.code == packet.code {
                        *status = SendStatus::Acked;
                    }
                }
            }
        }
        RecvRes::Break => return Ok(0),
        _ => {}
    }

    // remove finished task
    while resend_packets
        .front()
        .map_or(false, |(_, status)| matches!(status, SendStatus::Acked))
    {
        resend_packets.pop_front();
    }
    Ok(2)
}
