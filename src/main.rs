mod config;
mod connect;
mod packet;
mod stage;
use packet::MyPacket;
use stage::*;
use std::collections::{VecDeque, HashMap};
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::process;
use std::sync::mpsc;
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = config::Config::parse(&args.as_slice()[1..]);
    if let Err(e) = run(config) {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn run(cfg: config::Config) -> Result<(), Box<dyn Error>> {
    println!(
        "running as {}",
        if cfg.is_server { "server" } else { "client" }
    );
    let mut stage = 0;
    let mut conn = connect::Connection::new(&cfg);

    let (sx, rx) = mpsc::channel();
    let input = std::fs::read(cfg.input_filename())?;
    let mut file = std::fs::File::create(cfg.output_filename())?;
    let data = input.as_slice();
    thread::spawn(move || user_input(sx));

    let mut queue = VecDeque::new();
    let mut map = HashMap::new();
    let mut offset = 0;
    let mut end = false;
    loop {
        offset += update_queue(&mut queue, &data[offset..], &cfg, conn.seq, &mut end);
        stage = match stage {
            0 => stage0(&rx, &mut conn, &cfg)?,
            1 => stage1(&mut conn, &cfg)?,
            2 => stage2(&mut conn, &cfg, &mut queue)?,
            3 => stage3(&mut conn, &cfg)?,
            4 => stage4(&mut conn, &cfg, &mut file, &mut map)?,
            5 => stage5(&mut conn, &cfg)?,
            6 => stage6(&mut conn, &cfg)?,
            _ => panic!("invalid stage"),
        };
        if stage == 0 {
            offset = 0;
            queue.clear();
            map.clear();
        }
    }
}

fn user_input(sender: mpsc::Sender<String>) {
    let func = move || -> Result<(), Box<dyn Error>> {
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            // read input
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();
            sender.send(input.to_string())?;
        }
    };

    if let Err(e) = func() {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn update_queue(
    queue: &mut VecDeque<(MyPacket, SendStatus)>,
    data: &[u8],
    config: &config::Config,
    seq: u8,
    end: &mut bool
) -> usize {
    let limit = config.window.into();
    let once_data_len = 1024;
    let mut count = 0;
    let mut seq = seq + queue.len() as u8;

    while queue.len() < limit && (data.len() - count) > 0 {
        seq %= 100;
        let chunk_size = std::cmp::min(once_data_len, data.len() - count);
        let item = (
            MyPacket::with_data(data[count..count + chunk_size].to_vec(), seq),
            SendStatus::Ready,
        );
        queue.push_back(item);
        count += chunk_size;
        seq += 1;
    }
    if !*end && queue.len() < limit && data.len() == count {
        let end_pack = (
            MyPacket::with_code(seq),
            SendStatus::Ready,
        );
        println!("end of pack");
        queue.push_back(end_pack);
        *end = true;
    }

    count
}

#[test]
fn test_update_queue() {
    let mut queue = VecDeque::new();
    let data = std::fs::read("server_input.txt").unwrap();
    let cfg = config::Config {
        is_server: true,
        window: 10,
        seq_siz: 100,
        max_timeout: 10,
        single_timeout: std::time::Duration::from_millis(300),
        receive_rate: 0.8,
        send_rate: 1.0,
    };
    let mut seq = 0;
    let mut offset = 0;
    let mut expect_offset = 0;
    offset += update_queue(&mut queue, &data[offset..], &cfg, seq, &mut false);
    expect_offset += 1024 * 10;
    assert_eq!(queue.len(), 10);
    assert_eq!(offset, expect_offset);
    // won't update anything this time
    offset += update_queue(&mut queue, &data[offset..], &cfg, seq, &mut false);
    assert_eq!(queue.len(), 10);
    assert_eq!(offset, expect_offset);

    for _ in 0..3 {
        queue.pop_front();
    }
    offset += update_queue(&mut queue, &data[offset..], &cfg, seq, &mut false);
    expect_offset += 1024 * 3;
    assert_eq!(queue.len(), 10);
    assert_eq!(offset, expect_offset);

    for _ in 0..10 {
        queue.pop_front();
    }
    seq = 98;
    offset += update_queue(&mut queue, &data[offset..], &cfg, seq, &mut false);
    expect_offset += 1024 * 10;
    assert_eq!(queue.len(), 10);
    assert_eq!(offset, expect_offset);
    assert_eq!(queue.front().unwrap().0.code, 98);
    assert_eq!(queue.back().unwrap().0.code, 7);
}
