#[derive(Debug)]
enum Method {
    Get,
    Post,
    Connect,
}

pub enum ProxyType {
    Direct,
    Fish,
    Ban,
}

pub struct Rule {
    pub direct: Vec<String>,
    pub fish: Vec<String>,
    pub ban: Vec<String>,
}

pub fn get_filter(head: &RequestHeader, rule: &Rule) -> ProxyType {
    for d in &rule.direct {
        if head.host.starts_with(d) {
            return ProxyType::Direct;
        }
    }
    for d in &rule.fish {
        if head.host.starts_with(d) {
            return ProxyType::Fish;
        }
    }
    for d in &rule.ban {
        if head.host.starts_with(d) {
            return ProxyType::Ban;
        }
    }
    return ProxyType::Direct;
}

impl Method {
    fn from_str(s: &str) -> Option<Method> {
        match s {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "CONNECT" => Some(Self::Connect),
            _ => None,
        }
    }

    fn to_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Connect => "CONNECT",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct RequestHeader {
    method: Method,
    pub url: String,
    cookie: Vec<String>,
    pub host: String,
    cache_time: Option<String>,
}

pub struct RespondHeader {
    pub code: u8,
    pub length: u16,
    pub content: Vec<u8>,
}

impl RequestHeader {
    pub fn from(msg: &str) -> Option<RequestHeader> {
        let mut lines = msg.lines();
        let parts: Vec<&str> = lines.next()?.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let method = Method::from_str(parts[0])?;
        let url = parts[1].to_string();

        let mut host = None;
        let mut cookie = Vec::new();

        for line in lines {
            match line {
                _ if line.starts_with("Host: ") => {
                    host.get_or_insert_with(|| line["Host: ".len()..].to_string());
                }
                _ if line.starts_with("Cookie: ") => {
                    cookie.push(line["Cookie: ".len()..].to_string());
                }
                _ => {}
            }
        }

        Some(RequestHeader {
            method,
            url,
            host: host?,
            cookie,
            cache_time: None,
        })
    }

    pub fn construct(&self, crlf: bool) -> String {
        let brk = if crlf { "\r\n" } else { "\n" };
        let http_ver = "HTTP/1.1";
        let tail = vec![
            "User-Agent: Wget/1.21.4",
            "Accept: */*",
            "Accept-Encoding: identity",
            "Connection: Keep-Alive",
            "Proxy-Connection: Keep-Alive",
        ];
        let mut head = format!("{} {} {}{}", self.method.to_str(), self.url, http_ver, brk);
        head = head + "Host: " + &self.host + brk;
        for line in tail {
            head += line;
            head += brk;
        }
        if self.cache_time.is_some() {
            head += "If-Modified-Since: ";
            head += self.cache_time.as_ref().unwrap();
            head += brk;
        }
        head += brk;
        return head;
    }

    pub fn get_uniq_name(&self) -> String {
        format!("{}{}", self.host, self.url)
    }

    pub fn set_time(&mut self, t: String) {
        self.cache_time = Some(t);
    }

    pub fn get_time(&self) -> Option<&str> {
        self.cache_time.as_ref().map(|v| v.as_str())
    }
}

#[test]
fn test_construct() {
    let header = RequestHeader {
        host: String::from("182.43.76.137:6500"),
        method: Method::Get,
        url: String::from("/"),
        cookie: Vec::new(),
        cache_time: None,
    };
    println!("{}", header.construct(true));
    println!("{:?}", header);

    let header = RequestHeader {
        host: String::from("182.43.76.137:6500"),
        method: Method::Get,
        url: String::from("/"),
        cookie: Vec::new(),
        cache_time: Some(String::from("Wed, 21 Oct 2015 07:28:00 GMT")),
    };
    println!("{:?}", header.construct(true));
    println!("{:?}", header);
}
