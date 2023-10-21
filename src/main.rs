use socket2::{Domain, Socket, Type};
use std::io;
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;
use std::thread;
mod httpheader;
use std::fs;
use httpheader::*;

pub const RESPON_NAME: &str = "HTTP/1.1 200 OK\r
Content-Length: 4905\r
Content-Type: text/html; charset=utf-8\r
Date: Sat, 21 Oct 2023 00:59:19 GMT\r
Server: fishman\r
\r\n";

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
            if let Err(e) = run_connect(ns) {
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

fn get_slice(buf: &mut [MaybeUninit<u8>], siz: usize) -> &[u8] {
    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, siz) }
}

fn run_connect(sock: Socket) -> io::Result<()> {
    // recv local
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    let size = sock.recv(&mut buffer)?;

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    let head = HttpHeader::from(&content).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to resolve request head",
    ))?;

    let rule = Rule {
        direct: vec![],
        fish: vec![String::from("sjtu.edu.cn")],
        ban: vec![String::from("example.com")],
    };

    report_local(&head)?;
    match get_filter(&head, &rule) {
        ProxyType::Direct => connect_dir(sock, head)?,
        ProxyType::Fish => connect_fish(sock, head)?,
        _ => refuse(sock, head)?,
    }

    Ok(())
}

fn report_local(head: &HttpHeader) -> io::Result<()> {
    println!(
        "Local connection: \n Hosts: {}\n Url: {}\n",
        head.host, head.url
    );
    Ok(())
}

fn connect_dir(lsock: Socket, head: HttpHeader) -> io::Result<()> {
    println!("😄 Direct connection: {}", head.host);
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    // send remote
    let mut host = head.host.clone();
    if !host.contains(":") {
        host += ":80";
    }
    if host.ends_with(":443") {
        println!("😅 no support for https yet");
        return Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "Failed with https",
        ));
    }
    let remote_addr: SocketAddr = host
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "Cannot resolve address"))?;
    println!("resolve url: {:?}", remote_addr);
    let remote_sock = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    remote_sock.connect(&remote_addr.into())?;
    // send back local
    let _ = remote_sock.send(head.construct(true).as_bytes())?;
    let resize = remote_sock.recv(&mut buffer)?;
    let _ = lsock.send(get_slice(&mut buffer, resize))?;

    Ok(())
}

fn connect_fish(lsock: Socket, head: HttpHeader) -> io::Result<()> {
    println!("🎣 Fish connection: {}", head.host);
    lsock.send(RESPON_NAME.as_bytes())?;
    // read from filesystem and send
    let content = fs::read_to_string("./src/page/welcom.html")?;
    lsock.send(content.as_bytes())?;
    Ok(())
}

fn refuse(lsock: Socket, head: HttpHeader) -> io::Result<()> {
    println!("🚫 Refuse connection to: {}", head.host);
    lsock.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}

#[test]
fn test_addr() -> io::Result<()> {
    let remote_addr: SocketAddr = "github.com"
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "Cannot resolve address"))?;
    println!("{:?}", remote_addr);
    Ok(())
}
