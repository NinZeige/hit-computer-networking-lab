use socket2::{Domain, Socket, Type};
use std::error::Error;
use std::net::SocketAddr;
use std::process;
use std::sync::{Arc, RwLock};
use std::thread;

mod connect_manager;
mod httpheader;

fn main() {
    if let Err(e) = run() {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut cache = Arc::new(RwLock::new(connect_manager::read_cache()?));
    let mut threads = Vec::new();
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;

    let addr: SocketAddr = "127.0.0.1:6500".parse().unwrap();
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(5)?;

    // start monitor threads
    let max_thread = 500;
    for _ in 0..max_thread {
        let (ns, addr_in) = socket.accept()?;
        let map = cache.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = connect_manager::run_connect(ns, addr_in, map) {
                println!("Application Error: {e}");
            }
        });
        threads.push(handle);
    }

    for thr in threads {
        thr.join().unwrap();
    }

    Ok(())
}
