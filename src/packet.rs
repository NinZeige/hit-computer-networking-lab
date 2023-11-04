use chrono::prelude::*;

#[derive(Clone, Debug)]
pub struct MyPacket {
    pub code: u8,
    pub content: Vec<u8>,
}

pub const HS1: u8 = 200;
pub const HS2: u8 = 201;
pub const HS3: u8 = 202;
pub const TIM: u8 = 203;

impl MyPacket {
    pub fn from(data: &[u8]) -> Result<Vec<MyPacket>, &'static str> {
        if data.is_empty() {
            return Err("Empty data is not allow to construct a packet");
        }

        let mut res = Vec::new();
        let mut start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte == 0xFF {
                let segment = &data[start..=i];
                let packet = Self::from_single(segment)?;
                res.push(packet);
                start = i + 1;
            }
        }

        Ok(res)
    }

    fn from_single(data: &[u8]) -> Result<MyPacket, &'static str> {
        if data.len() < 2 || data[data.len() - 1] != 0xFF {
            return Err("Wrong data format");
        }
        Ok(MyPacket {
            code: data[0],
            content: data[1..data.len() - 1].to_vec(),
        })
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(self.content.len() + 1);
        res.push(self.code);
        res.append(&mut self.content.clone());
        res.push(0xFF);
        res
    }
    pub fn with_code(code: u8) -> MyPacket {
        MyPacket {
            code,
            content: Vec::new(),
        }
    }

    pub fn with_data(data: Vec<u8>, seq: u8) -> MyPacket {
        MyPacket {
            code: seq,
            content: data,
        }
    }

    pub fn with_now() -> MyPacket {
        // read time now & send
        let now = Local::now().to_string();
        Self::with_data(now.as_bytes().to_vec(), TIM)
    }
}

#[test]
fn test_from() {
    let data: Vec<u8> = vec![200, 115, 114, 113, 255];
    println!("{:?}", MyPacket::from_single(data.as_slice()));
    println!("{:?}", MyPacket::from(data.as_slice()));

    let data: Vec<u8> = vec![200, 255];
    println!("{:?}", MyPacket::from_single(data.as_slice()));
    println!("{:?}", MyPacket::from(data.as_slice()));
    let data: Vec<u8> = vec![200, 255, 204, 255, 127, 100, 122, 33, 255];
    println!("{:?}", MyPacket::from(data.as_slice()));
    let pkts = MyPacket::from(data.as_slice()).unwrap();
    let mut bytes = Vec::new();
    for pkt in pkts {
        bytes.push(pkt.to_vec());
    }
    println!("{:?}", bytes);
    println!(
        "convert to and back: \n{:?}",
        MyPacket::from(data.as_slice())
    );
}

#[test]
fn test_with() {
    println!("{:?}", MyPacket::with_data(vec![1, 2, 3, 4, 5], 102));
}

#[test]
fn test_time() {
    let p = MyPacket::with_now();
    println!("{:?}", p);
    println!("{:?}", String::from_utf8_lossy(p.content.as_ref()));
}
