use socket2::{Domain, Socket, Type};
use std::io;
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;
use std::thread;
mod httpheader;

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
    let max_thread = 5;
    for _ in 0..max_thread {
        let (ns, _) = socket.accept()?;
        let handle = thread::spawn(move || handle_single(ns));
        threads.push(handle);
    }

    for thr in threads {
        thr.join().unwrap()?;
    }

    Ok(())
}

fn handle_single(sock: Socket) -> io::Result<()> {
    // recv local
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    let size = sock.recv(&mut buffer)?;

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    let head = httpheader::HttpHeader::from(&content).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to resolve request head",
    ))?;
    
    // send remote
    let remote_addr: SocketAddr = head.host.to_socket_addrs()?.next().ok_or(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "Cannot resolve address",
    ))?;

    let remote_sock = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    remote_sock.connect(&remote_addr.into())?;
    remote_sock.send(buffer);

    Ok(())
}
