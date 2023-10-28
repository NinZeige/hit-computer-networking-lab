mod config;
mod packet;
mod stage;
mod recv_res;
use packet::MyPacket;
use stage::*;
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::net;
use std::process;
use std::sync::mpsc;
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = config::Config::parse(&args.as_slice()[1..]);
    if let Err(e) = if config.is_server {
        run_server(config)
    } else {
        run_client(config)
    } {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn run_client(cfg: config::Config) -> Result<(), Box<dyn Error>> {
    println!("running as client");
    Ok(())
}

fn run_server(cfg: config::Config) -> Result<(), Box<dyn Error>> {
    println!("running as server");
    let mut stage = 0;
    let addr: net::SocketAddr = "127.0.0.1:6500".parse()?;
    let sock = net::UdpSocket::bind(addr)?;
    let (sx, rx) = mpsc::channel();
    let file_cotent: Vec<u8> = Vec::new(); // somehow we read this file
    thread::spawn(move || user_input(sx));
    let data = file_cotent.as_slice();

    let mut try_times = 0;
    let mut queue = VecDeque::new();
    let mut index = 0;
    loop {
        index += update_queue(&mut queue, &data[index..], &cfg, &addr);
        stage = match stage {
            1 => stage1(&sock, &mut try_times, &MyPacket::with_code(200, &addr))?,
            2 => stage2(&sock, &mut try_times, &mut queue)?,
            _ => stage0(&sock, &rx, &mut try_times)?,
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
    src: &net::SocketAddr,
) -> usize {
    let limit = config.window;
    let once_data_len = 1024;
    let mut count = 0;

    while queue.len() < limit && (data.len() - count) > 0 {
        let chunk_size = std::cmp::min(once_data_len, data.len() - count);
        let item = (
            MyPacket::with_data(data[count..count + chunk_size].to_vec(), src),
            SendStatus::Ready,
        );
        queue.push_back(item);
        count += chunk_size;
    }

    count
}
