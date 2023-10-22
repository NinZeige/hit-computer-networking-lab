use socket2::{Domain, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::process;
use std::thread;

mod httpheader;
mod connect_manager;


fn main() {
    if let Err(e) = run() {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut threads = Vec::new();
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;

    let addr: SocketAddr = "127.0.0.1:6500".parse().unwrap();
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(5)?;

    // start monitor threads
    let max_thread = 500;
    for _ in 0..max_thread {
        let (ns, _) = socket.accept()?;
        let handle = thread::spawn(move || {
            if let Err(e) = connect_manager::run_connect(ns) {
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

