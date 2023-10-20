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

    // helpers, handle with MaybeUninit
    let get_slice = |buf: &mut [MaybeUninit<u8>], siz| unsafe {
        std::slice::from_raw_parts(buf.as_ptr() as *const u8, siz)
    };

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    let head = httpheader::HttpHeader::from(&content).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to resolve request head",
    ))?;
    report_local(&head)?;

    // send remote
    let remote_addr: SocketAddr = head.host.to_socket_addrs()?.next().ok_or(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "Cannot resolve address",
    ))?;
    let remote_sock = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    remote_sock.connect(&remote_addr.into())?;

    // send back local
    let _ = remote_sock.send(get_slice(&mut buffer, size))?;
    let mut resize = 0;
    loop {
        let size = remote_sock.recv(&mut buffer[resize..])?;
        resize += size;
        if size == 0 {
            break;
        }
    }
    let _ = sock.send(get_slice(&mut buffer, resize))?;

    Ok(())
}


fn report_local(head: &httpheader::HttpHeader) -> io::Result<()> {
    println!("Local connection: \n Hosts: {}\n Url: {}\n\n", head.host, head.url);
    Ok(())
}