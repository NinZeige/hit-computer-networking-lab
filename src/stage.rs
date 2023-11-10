use crate::config::*;
use crate::connect::*;
use crate::packet::{self, MyPacket};
use std::collections::{VecDeque, HashMap};
use std::error::Error;
use std::io::Write;
use std::process;
use std::sync::mpsc;
use std::time;

pub enum SendStatus {
    Ready,
    OnFly(time::Instant),
    Acked,
}

fn handle_break(conn: &mut Connection) -> u32 {
    println!("connection break");
    conn.timeout = 0;
    conn.seq = 0;
    conn.est = false;
    0
}

fn cycle_less_than(a: u8, b: u8, cfg: &Config) -> bool {
    if a < b {
        b - a <= cfg.window
    } else {
        a - b < cfg.seq_siz && a - b >= cfg.seq_siz - cfg.window
    }
}

pub fn stage0(
    queue: &mpsc::Receiver<String>,
    conn: &mut Connection,
    cfg: &Config,
) -> Result<u32, Box<dyn Error>> {
    // handle with local input
    if let Ok(s) = queue.try_recv() {
        return match s.as_str() {
            "quit" => process::exit(0),
            "send" => {
                let pkt = MyPacket::with_code(packet::HS1);
                try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                Ok(3)
            }
            "time" => {
                let pkt = MyPacket::with_code(packet::TIM);
                try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                Ok(5)
            }
            _ => {
                println!("unrecognized command");
                Ok(0)
            }
        };
    }

    // handle with network request
    let recv_res = try_recv_lossy(&conn.local, conn.timeout, cfg)?;
    if let RecvRes::Get(pkts) = recv_res {
        conn.timeout = 0;
        // drop all previous packet
        if let Some(pkt) = pkts.last() {
            return match pkt.code {
                packet::HS1 => {
                    println!("handshake1");
                    let pkt = MyPacket::with_code(packet::HS2);
                    try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                    Ok(1)
                }
                packet::TIM => {
                    println!("time req");
                    let pkt = MyPacket::with_now();
                    try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                    Ok(6)
                }
                _ => Ok(0),
            };
        }
    }

    Ok(0)
}

pub fn stage1(conn: &mut Connection, cfg: &Config) -> Result<u32, Box<dyn Error>> {
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Timeout => {
            conn.timeout += 1;
            let pkt = MyPacket::with_code(packet::HS2);
            try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
            Ok(1)
        }
        RecvRes::Break => Ok(handle_break(conn)),
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            if let Some(pkt) = pkts.last() {
                if pkt.code == packet::HS3 {
                    println!("handshake3");
                    return Ok(2);
                }
            }
            Ok(0)
        }
    }
}

pub fn stage2(
    conn: &mut Connection,
    cfg: &Config,
    resend_packets: &mut VecDeque<(MyPacket, SendStatus)>,
) -> Result<u32, Box<dyn Error>> {
    // 1. check if finished
    if resend_packets.is_empty() {
        println!("send finished");
        return Ok(0);
    }

    // check timeout first to avoid repeat send
    for (_, status) in resend_packets.iter_mut() {
        if let SendStatus::OnFly(now) = status {
            if now.elapsed() > cfg.single_timeout {
                println!("set timeout");
                *status = SendStatus::Ready;
            }
        }
    }

    // 2. send packet
    let mut pkts = Vec::new();
    for (pkt, status) in resend_packets.iter_mut() {
        if let SendStatus::Ready = status {
            *status = SendStatus::OnFly(time::Instant::now());
            pkts.push(pkt.clone());
        }
    }
    println!("resend package: {:?}", pkts.len());
    try_send_lossy(&conn, pkts.iter().collect(), cfg.send_rate)?;

    // 3. recv ack
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Break => return Ok(handle_break(conn)),
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            for pkt in pkts {
                for (packet, status) in resend_packets.iter_mut() {
                    if let SendStatus::OnFly(_) = status {
                        let ack_code = pkt.code - cfg.seq_siz;
                        if ack_code == packet.code {
                            println!("Acked: {}", packet.code);
                            *status = SendStatus::Acked;
                            break;
                        } else if cycle_less_than(ack_code, conn.seq, cfg) {
                            break;
                        }
                    }
                }
            }
        }
        RecvRes::Timeout => conn.timeout += 1,
    }

    // 4. remove finished task
    while resend_packets
        .front()
        .map_or(false, |(_, status)| matches!(status, SendStatus::Acked))
    {
        resend_packets.pop_front();
        conn.seq = (conn.seq + 1) % cfg.seq_siz;
    }

    Ok(2)
}

pub fn stage3(conn: &mut Connection, cfg: &Config) -> Result<u32, Box<dyn Error>> {
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Timeout => {
            conn.timeout += 1;
            let pkt = MyPacket::with_code(packet::HS1);
            try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
            Ok(3)
        }
        RecvRes::Break => Ok(handle_break(conn)),
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            if let Some(pkt) = pkts.last() {
                if pkt.code == packet::HS2 {
                    println!("handshake2");
                    let pkt = MyPacket::with_code(packet::HS3);
                    try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                    return Ok(4);
                }
            }
            Ok(0)
        }
    }
}

pub fn stage4(
    conn: &mut Connection,
    cfg: &Config,
    file: &mut std::fs::File,
    map: &mut HashMap<u8, Vec<u8>>
) -> Result<u32, Box<dyn Error>> {
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Timeout => {
            if !conn.est {
                let pkt = MyPacket::with_code(packet::HS3);
                try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
            }
            conn.timeout += 1;
            Ok(4)
        }
        RecvRes::Break => Ok(handle_break(conn)),
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            conn.est = true;
            let mut to_ack = Vec::new();
            for pkt in pkts {
                    to_ack.push(pkt.clone());
                    map.insert(pkt.code, pkt.content.to_owned());
                }
            if !to_ack.is_empty() {
                let codes: Vec<MyPacket> = to_ack
                    .iter()
                    .filter_map(|pkt| {
                        if pkt.code < 100 {
                            // 当 pkt.code 小于 100 时，返回一个 code + 100 的包
                            Some(MyPacket::with_code(pkt.code + 100))
                        } else {
                            // 对于其他情况，跳过这些包
                            None
                        }
                    })
                    .collect();
                try_send_lossy(&conn, codes.iter().map(|v| v).collect(), cfg.send_rate)?;
            }
            loop {
                match map.remove(&conn.seq) {
                    Some(x) => {
                        if x.len() == 0 {
                            println!("recv finished");
                            return Ok(0);
                        }
                        file.write(x.as_slice())?;
                        conn.seq = (conn.seq + 1) % cfg.seq_siz;
                    },
                    None => break,
                }
            }
            Ok(4)
        }
    }
}

pub fn stage5(conn: &mut Connection, cfg: &Config) -> Result<u32, Box<dyn Error>> {
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Timeout => {
            conn.timeout += 1;
            let pkt = MyPacket::with_code(packet::TIM);
            try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
            Ok(5)
        }
        RecvRes::Break => Ok(handle_break(conn)),
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            if let Some(pkt) = pkts.last() {
                if pkt.code == packet::TIM {
                    println!("Time: {}", String::from_utf8(pkt.content.clone())?);
                    return Ok(0);
                }
            }
            Ok(5)
        }
    }
}

pub fn stage6(conn: &mut Connection, cfg: &Config) -> Result<u32, Box<dyn Error>> {
    match try_recv_lossy(&conn.local, conn.timeout, cfg)? {
        RecvRes::Timeout => {
            conn.timeout += 1;
            Ok(6)
        }
        RecvRes::Break => {
            Ok(handle_break(conn))
        }
        RecvRes::Get(pkts) => {
            conn.timeout = 0;
            for pkt in pkts {
                if pkt.code == packet::TIM {
                    let pkt = MyPacket::with_now();
                    try_send_lossy(&conn, vec![&pkt], cfg.send_rate)?;
                }
            }
            Ok(6)
        }
    }
}

#[test]
fn test_stage0() -> Result<(), Box<dyn Error>> {
    use std::net::{SocketAddr, UdpSocket};
    use std::thread;

    let local_addr = "127.0.0.1:6500".parse::<SocketAddr>().unwrap();
    let remote_addr = "127.0.0.1:6501".parse::<SocketAddr>().unwrap();
    let remote_sock = UdpSocket::bind(remote_addr.clone()).unwrap();
    let (sx, rx) = mpsc::channel::<String>();

    let handle = thread::spawn(|| {
        let mut cfg = Config::parse(vec![String::from("--server")].as_ref());
        cfg.receive_rate = 1.0; // simulate good network connection
        let rx = rx;
        let mut con = Connection::new(&cfg);
        let dur = time::Duration::from_millis(100);

        // 1. test packet::HS1 (handshake)
        let res = stage0(&rx, &mut con, &cfg).unwrap();
        assert_eq!(res, 1);
        // 2. test packet::TIM (time send)
        let res = stage0(&rx, &mut con, &cfg).unwrap();
        assert_eq!(res, 6);
        // 3. test other (drop meaningless)
        let res = stage0(&rx, &mut con, &cfg).unwrap();
        assert_eq!(res, 0);
        // 4. test time
        thread::sleep(dur.clone());
        let res = stage0(&rx, &mut con, &cfg).unwrap();
        assert_eq!(res, 5);
        // 4. test send
        thread::sleep(dur.clone());
        let res = stage0(&rx, &mut con, &cfg).unwrap();
        assert_eq!(res, 3);
    });

    let dur = time::Duration::from_millis(100);
    // 1. test packet::HS1
    thread::sleep(dur.clone());
    let pkt = MyPacket::with_code(packet::HS1);
    try_send(&remote_sock, &local_addr, vec![&pkt])?;
    // 2. test packet::TIM
    thread::sleep(dur.clone());
    let pkt = MyPacket::with_code(packet::TIM);
    try_send(&remote_sock, &local_addr, vec![&pkt])?;
    // 3. test other
    thread::sleep(dur.clone());
    let pkt = MyPacket::with_code(210);
    try_send(&remote_sock, &local_addr, vec![&pkt])?;
    // 4. send time
    let sxx = sx.clone();
    sxx.send(String::from("time")).unwrap();
    // 5. test send
    thread::sleep(dur.clone());
    sxx.send(String::from("send")).unwrap();

    handle.join().unwrap();
    Ok(())
}

#[test]
fn test_main() {
    // core functionality test
    // test GBN protocol
    use std::thread;

    let handle = thread::spawn(|| {
        let mut cfg = Config::parse(vec![String::from("--server")].as_ref());
        cfg.max_timeout = 1000;
        cfg.seq_siz = 10;
        cfg.window = 5;
        let mut conn = Connection::new(&cfg);
        conn.seq = 8;
        let mut pkt: VecDeque<(MyPacket, SendStatus)> = (8..10)
            .map(|seq| MyPacket::with_data(vec![61, 62, 63, 64, 65], seq))
            .map(|pkt| (pkt, SendStatus::Ready))
            .collect();
        pkt.append(
            &mut (0..2)
                .map(|seq| MyPacket::with_data(vec![61, 62, 63, 64, 65], seq))
                .map(|pkt| (pkt, SendStatus::Ready))
                .collect::<VecDeque<_>>(),
        );
        pkt.push_back((MyPacket::with_data(Vec::new(), 2), SendStatus::Ready));
        loop {
            let stage = stage2(&mut conn, &cfg, &mut pkt).unwrap_or_else(|e| {
                println!("{:?}", e);
                2
            });
            if stage == 0 {
                break;
            }
        }
    });

    let mut cfg = Config::parse(vec![String::from("--client")].as_ref());
    cfg.max_timeout = 1000;
    cfg.seq_siz = 10;
    cfg.window = 5;
    let mut conn = Connection::new(&cfg);
    conn.seq = 8;
    let mut file = std::fs::File::create(cfg.output_filename()).unwrap();
    let mut map = HashMap::new();
    loop {
        let stage = stage4(&mut conn, &cfg, &mut file, &mut map).unwrap_or_else(|e| {
            println!("{:?}", e);
            4
        });
        if stage == 0 {
            break;
        }
    }

    if let Err(e) = handle.join() {
        println!("Application Error: {:?}", e);
    }
}

#[test]
fn test_time() {
    use std::thread;

    let handle = thread::spawn(|| {
        let cfg = Config::parse(vec![String::from("--server")].as_ref());
        let mut conn = Connection::new(&cfg);
        loop {
            let stage = stage6(&mut conn, &cfg).unwrap_or(6);
            if stage == 0 {
                break;
            }
        }
    });

    let cfg = Config::parse(vec![String::from("--client")].as_ref());
    let mut conn = Connection::new(&cfg);
    loop {
        let stage = stage5(&mut conn, &cfg).unwrap();
        if stage == 0 {
            break;
        }
    }

    if let Err(e) = handle.join() {
        println!("Application Error: {:?}", e);
    }
}
