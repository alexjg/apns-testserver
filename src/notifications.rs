use serialize::{Encodable, Encoder, Decodable};
use serialize::hex::{ToHex};
use std::num::{from_u16};
use std::io::{Reader};

static CHARS: &'static[u8] = b"0123456789abcdef";
pub struct U8Vec(pub Vec<u8>);

impl ToHex for U8Vec {
    fn to_hex(&self) -> String {
        let &U8Vec(ref wrapped) = self;
        let mut v = Vec::with_capacity(wrapped.len() * 2);
        for &byte in wrapped.iter() {
            v.push(CHARS[(byte >> 4) as uint]);
            v.push(CHARS[(byte & 0xf) as uint]);
        }
        return String::from_utf8(v).unwrap()
    }
}

struct APNSHeader {
    command: u8,
    frame_length: u32,
}

#[deriving(Encodable, Clone)]
pub struct Notification {
    device_token: String,
    payload: String,
    identifier: u32,
    expiration_date: u32,
    priority: u8,
}

pub struct NotificationReader<'a, T: 'a> {
    //reader: &'a mut BufferedReader<T>,
    reader: &'a mut T,
    current_frame_position: int,
    current_device_token: Option<String>,
    current_payload: Option<String>,
    current_identifier: Option<u32>,
    current_expiration_date: Option<u32>,
    current_priority: Option<u8>,
}

impl<'a, T: Reader> NotificationReader<'a, T> {
    pub fn new(reader: &'a mut T) -> NotificationReader<'a, T> {
        NotificationReader {
            reader: reader,
            current_frame_position: 0,
            current_device_token: None,
            current_payload: None,
            current_identifier: None,
            current_expiration_date: None,
            current_priority: None,
        }
    }

    fn read_header(&mut self) -> APNSHeader {
        let command = self.reader.read_byte().unwrap();
        let frame_length = self.reader.read_be_u32().unwrap();
        let header = APNSHeader{command: command, frame_length: frame_length};
        return header;
    }

    fn read_item_length(&mut self) -> uint {
        let result = self.reader.read_be_u16();
        self.current_frame_position += 2;
        return from_u16(result.unwrap()).unwrap();
    }

    fn read_device_token(&mut self) {
        let item_length = self.read_item_length();
        let raw = U8Vec(self.reader.read_exact(item_length).unwrap());
        let device_token = raw.to_hex();
        self.current_device_token = Some(device_token.clone());
        debug!("Read device token {0}", device_token.clone());
        self.current_frame_position += item_length as int;
    }

    fn read_identifier(&mut self) {
        self.read_item_length();
        self.current_identifier = Some(self.reader.read_be_u32().unwrap());
        debug!("Read identifier {0}", self.current_identifier.unwrap());
        self.current_frame_position += 4;
    }

    fn read_expiration(&mut self) {
        self.read_item_length();
        self.current_expiration_date = Some(self.reader.read_be_u32().unwrap());
        debug!("Read expiration date {0}", self.current_expiration_date.unwrap());
        self.current_frame_position += 4;
    }

    fn read_priority(&mut self) {
        self.read_item_length();
        self.current_priority = Some(self.reader.read_u8().unwrap());
        debug!("Read priority {0}", self.current_priority.unwrap());
        self.current_frame_position += 1;
    }

    fn read_payload(&mut self) {
        let item_length = self.read_item_length();
        let mut payload: Vec<u8>  = Vec::with_capacity(item_length as uint);
        for _ in range(0, item_length) {
            payload.push(self.reader.read_u8().unwrap());
        }
        let payload_str = String::from_utf8(payload.to_vec()).unwrap();
        debug!("Read payload {0}", payload_str);
        self.current_frame_position += item_length as int;
        self.current_payload = Some(payload_str);
    }

    fn reset(&mut self) {
        self.current_device_token = None;
        self.current_payload = None;
        self.current_identifier = None;
        self.current_expiration_date = None;
        self.current_priority = None;
    }

    pub fn read_notification(&mut self) -> Option<Notification> {
        self.reset();
        self.current_frame_position = 0;
        let header:APNSHeader = self.read_header();
        info!("Read header, command: {0}, data length: {1}", header.command, header.frame_length);

        while self.current_frame_position < (header.frame_length - 1) as int {
            let item_id = self.reader.read_u8();
            self.current_frame_position += 1;

            match item_id.unwrap() {
                1u8 => self.read_device_token(),
                2u8 => self.read_payload(),
                3u8 => self.read_identifier(),
                4u8 => self.read_expiration(),
                5u8 => self.read_priority(),
                other_id => fail!("Unknown item id {}", other_id)
            }
        }
        return Some(
            Notification {
                device_token: self.current_device_token.clone().unwrap(),
                payload: self.current_payload.clone().unwrap(),
                identifier: self.current_identifier.unwrap(),
                expiration_date: self.current_expiration_date.unwrap(),
                priority: self.current_priority.unwrap(),
            }
        )
    }
}

