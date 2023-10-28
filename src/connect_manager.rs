use crate::httpheader::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use socket2::{Domain, SockAddr, Socket, Type};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind};
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};

fn get_slice(data: &[MaybeUninit<u8>], len: usize) -> &[u8] {
    unsafe {
        let ptr = data.as_ptr() as *const u8;
        std::slice::from_raw_parts(ptr, len)
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub content: Vec<u8>,
    pub time: String,
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
    let _ = addr.as_socket_ipv4().ok_or(io::Error::new(
        ErrorKind::AddrNotAvailable,
        "Not IPv4 connection",
    ))?;
    // recv local
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    let size = sock.recv(&mut buffer)?;

    // parse what socket read to string
    let content: String = (0..size)
        .map(|i| unsafe { buffer[i].assume_init() as char })
        .collect();
    let mut head = RequestHeader::from(&content).ok_or(io::Error::new(
        ErrorKind::InvalidData,
        "Failed to resolve request head",
    ))?;

    let cache_ent = cache
        .read()
        .unwrap()
        .get(&head.get_uniq_name())
        .map(|v| v.clone());
    // check if already cached
    let mut cache_str = None;
    if let Some(ent) = cache_ent {
        cache_str = Some(ent.content);
        head.set_time(ent.time);
    }

    let rule = Rule {
        direct: vec![],
        fish: vec![String::from("sjtu.edu.cn")],
        ban: vec![String::from("example.com")],
    };

    match get_filter(&head, &rule) {
        ProxyType::Direct => connect_dir(sock, head, cache, cache_str)?,
        ProxyType::Fish => connect_fish(sock, head)?,
        _ => refuse(sock, head)?,
    }

    Ok(())
}

fn recv_fullhttp(buffer: &mut [MaybeUninit<u8>], rsock: Socket) -> io::Result<RequestHeader>{
    // get http respond head
    let resize = rsock.recv(&mut buffer)?;
    let trunk = get_slice(&buffer, resize);
    let head = String::from_utf8_lossy(trunk);
    if let Some(x) = head.find("\r\n\r\n") {
        
        Ok(())
    } else {
        Err(io::Error::new(ErrorKind::InvalidData, "cannot resolve http response"))
    }
}

fn connect_dir(
    lsock: Socket,
    head: RequestHeader,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    cache_str: Option<Vec<u8>>,
) -> io::Result<()> {
    println!("😄 Direct connection: {}", head.host);
    let mut buffer: [MaybeUninit<u8>; 65515] = unsafe { MaybeUninit::uninit().assume_init() };
    // send remote
    let host = if head.host.contains(":") {
        head.host.to_string()
    } else {
        format!("{}:80", head.host)
    };
    if host.ends_with(":443") {
        println!("🔐 uh-oh, no support for https yet");
        return Err(io::Error::new(
            ErrorKind::AddrNotAvailable,
            "Failed with https",
        ));
    }

    let remote_addr: SocketAddr = host
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(ErrorKind::AddrNotAvailable, "Cannot resolve address"))?;
    println!("resolve url: {:?}", remote_addr);
    let remote_sock = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    remote_sock.connect(&remote_addr.into())?;
    remote_sock.send(head.construct(true).as_bytes())?;

    let resize = remote_sock.recv(&mut buffer)?;
    let raw_buff = get_slice(&buffer, resize);

    if head.get_time().is_some() {
        println!("🔍 find cache: true");

        let recv_str: String = String::from_utf8_lossy(raw_buff).into_owned();
        if let Some(line) = recv_str.lines().next() {
            if let Some(code) = line.split_ascii_whitespace().nth(1) {
                return match code {
                    "304" if cache_str.is_some() => {
                        println!("network return not-modified");
                        lsock.send(cache_str.unwrap().as_slice())?;
                        Ok(())
                    }
                    _ => update_and_send(lsock, cache, raw_buff, head.url),
                };
            }
        }
        Err(io::Error::new(
            ErrorKind::InvalidData,
            "Invalid http response",
        ))
    } else {
        println!("🔍 find cache: false");
        update_and_send(lsock, cache, raw_buff, head.url)
    }
}

fn update_and_send(
    lsock: Socket,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    recv_str: &[u8],
    url: String,
) -> io::Result<()> {
    cache.write().unwrap().insert(
        url,
        CacheEntry {
            content: recv_str.into_iter().map(|&v| v.clone()).collect(),
            time: get_gmttime(),
        },
    );
    lsock.send(recv_str).map(|_| ())
}

fn connect_fish(lsock: Socket, head: RequestHeader) -> io::Result<()> {
    println!("🎣 Fish connection: {}", head.host);
    lsock.send(RESPON_NAME.as_bytes())?;
    // read from filesystem and send
    let content = fs::read_to_string("./src/page/welcom.html")?;
    lsock.send(content.as_bytes())?;
    Ok(())
}

fn refuse(lsock: Socket, head: RequestHeader) -> io::Result<()> {
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
                        content: tmpv,
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
    let content = String::from("This is good");

    map.insert(
        String::from("http://www.wenku8.net"),
        CacheEntry {
            content: content.into_bytes(),
            time: String::from("Pretend to be time"),
        },
    );
    map.insert(
        String::from("http://182.43.76.137"),
        CacheEntry {
            content: String::from("This is right").into_bytes(),
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
