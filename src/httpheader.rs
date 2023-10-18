#[derive(Debug)]
enum Method {
    Get,
    Post,
    Connect,
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

#[derive(Debug)]
pub struct HttpHeader {
    method: Method,
    pub url: String,
    cookie: Vec<String>,
    pub host: String,
}

impl HttpHeader {
    pub fn from(msg: &str) -> Option<HttpHeader> {
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

        Some(HttpHeader {
            method,
            url,
            host: host?,
            cookie,
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
        head = head + "Host: " + self.host.as_str() + brk;
        for line in tail {
            head += line;
            head += brk;
        }
        return head;
    }
}
