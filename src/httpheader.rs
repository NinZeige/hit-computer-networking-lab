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
}

#[derive(Debug)]
pub struct HttpHeader {
    method: Method,
    url: String,
    cookie: Vec<String>,
    host: String,
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
}
