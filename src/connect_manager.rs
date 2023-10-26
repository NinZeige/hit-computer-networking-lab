use crate::httpheader::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use socket2::{Domain, SockAddr, Socket, Type};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct CacheEntry {
    pub content: String,
    pub time: String,
}

fn get_slice(buf: &mut [MaybeUninit<u8>], siz: usize) -> &[u8] {
    unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, siz) }
}

pub const RESPON_NAME: &str = "HTTP/1.1 200 OK\r
Content-Length: 4905\r
Content-Type: text/html; charset=utf-8\r
Date: Sat, 21 Oct 2023 00:59:19 GMT\r
Server: fishman\r
\r\n";

pub fn run_connect(
    sock: Socket,
    addr: SockAddr,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
) -> Result<(), Box<dyn Error>> {
    let addr = addr.as_socket_ipv4().ok_or(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "Not IPv4 connection",
    ))?;
    // recv local
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    let size = sock.recv(&mut buffer)?;

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    let mut head = HttpHeader::from(&content).ok_or(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to resolve request head",
    ))?;

    // check if already cached
    head.set_time(
        cache
            .read()
            .unwrap()
            .get(head.get_uniq_name().as_str())
            .map(|v| v.time.clone()),
    );

    let rule = Rule {
        direct: vec![],
        fish: vec![String::from("sjtu.edu.cn")],
        ban: vec![String::from("example.com")],
    };

    match get_filter(&head, &rule) {
        ProxyType::Direct => connect_dir(sock, head, cache)?,
        ProxyType::Fish => connect_fish(sock, head)?,
        _ => refuse(sock, head)?,
    }

    Ok(())
}

fn connect_dir(lsock: Socket, head: HttpHeader, cache: Arc<RwLock<HashMap<String, CacheEntry>>>) -> io::Result<()> {
    println!("😄 Direct connection: {}", head.host);
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    // send remote
    let mut host = head.host.clone();
    if !host.contains(":") {
        host += ":80";
    }
    if host.ends_with(":443") {
        println!("🔐 uh-oh, no support for https yet");
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

    let _ = remote_sock.send(head.construct(true).as_bytes())?;
    let resize = remote_sock.recv(&mut buffer)?;
    if let Some(_) = head.get_time() {
        println!("🔍 find cache: true");
        
    } else {
        println!("🔍 find cache: false")
    }

    // send back local
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

pub fn read_cache() -> Result<HashMap<String, CacheEntry>, Box<dyn Error>> {
    let content = fs::read_to_string("./cache.txt")?;

    let mut res = HashMap::with_capacity(content.lines().count());
    for line in content.lines() {
        let mut words = line.split_whitespace();
        if let (Some(k), Some(v), Some(t)) = (words.next(), words.next(), words.next()) {
            if let (Ok(tmpk), Ok(tmpv), Ok(tmpt)) =
                (STANDARD.decode(k), STANDARD.decode(v), STANDARD.decode(t))
            {
                res.insert(
                    String::from_utf8_lossy(&tmpk).into_owned(),
                    CacheEntry {
                        content: String::from_utf8_lossy(&tmpv).into_owned(),
                        time: String::from_utf8_lossy(&tmpt).into_owned(),
                    },
                );
            }
        }
    }
    Ok(res)
}

pub fn write_cahce(cache: HashMap<String, CacheEntry>) -> io::Result<()> {
    let mut contents = String::new();

    for (key, value) in cache {
        let (k, v, t) = (
            STANDARD.encode(key),
            STANDARD.encode(value.content),
            STANDARD.encode(value.time),
        );
        contents.push_str(&k);
        contents.push(' ');
        contents.push_str(&v);
        contents.push(' ');
        contents.push_str(&t);
        contents.push('\n');
    }
    fs::write("./cache.txt", contents)
}

#[test]
fn test_get_store() {
    let mut map = HashMap::new();
    map.insert(
        String::from("http://www.wenku8.net"),
        CacheEntry {
            content: String::from("This is good"),
            time: String::from("Pretend to be time"),
        },
    );
    map.insert(
        String::from("http://182.43.76.137"),
        CacheEntry {
            content: String::from("This is right"),
            time: String::from("pre to be time"),
        },
    );
    println!("{:?}", map);
    if let Err(e) = write_cahce(map) {
        println!("write error: {:?}", e);
    }
    println!("{:?}", read_cache());
}

#[test]
fn test_only_read() {
    println!("{:?}", read_cache());
}

fn get_gmttime() -> String {
    Utc::now().format("%a, %d %b %Y %T GMT").to_string()
}
