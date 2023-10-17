use socket2::{Domain, Socket, Type};
use std::io;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::process;
mod httpheader;

fn main() {
    if let Err(e) = run() {
        println!("Application Error: {e}");
        process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;

    let addr: SocketAddr = "127.0.0.1:6500".parse().unwrap();
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(5)?;
    let (ns, _) = socket.accept()?;
    let mut buffer: [MaybeUninit<u8>; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
    let size = ns.recv(&mut buffer)?;

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    println!("{content}");
    let head = httpheader::HttpHeader::from(&content);
    println!("{:?}", head);

    Ok(())
}
