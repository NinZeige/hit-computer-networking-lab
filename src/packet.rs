use std::net::SocketAddr;

#[derive(Clone)]
pub struct MyPacket {
    pub code: u8,
    pub content: Vec<u8>,
    pub src: SocketAddr,
}

impl MyPacket {
    pub fn from(data: &[u8], src: SocketAddr) -> MyPacket {
        if data.len() == 0 {
            panic!("Empty data is not allow to construct a packet");
        }
        MyPacket {
            code: data[0],
            content: data[1..].to_vec(),
            src,
        }
    }
    pub fn to_vec(mut self) -> Vec<u8> {
        let mut res = Vec::with_capacity(self.content.len() + 1);
        res.push(self.code);
        res.append(&mut self.content);
        res.push(0x1A);
        res
    }
    pub fn with_code(code: u8, src: &SocketAddr) -> MyPacket {
        MyPacket {
            code,
            content: Vec::new(),
            src: *src,
        }
    }

    pub fn with_data(data: Vec<u8>, src: &SocketAddr) -> MyPacket {
        MyPacket {
            code: 200,
            content: data,
            src: *src,
        }
    }
}
