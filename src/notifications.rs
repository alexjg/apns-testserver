use serialize::json::{Json};
use serialize::{Encodable, Encoder};
use serialize::hex::{ToHex};
use std::num::{from_u16};
use std::io::{Reader, IoError};

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
    payload: Json,
    identifier: u32,
    expiration_date: u32,
    priority: u8,
}

pub enum NotificationReadError {
    NotificationReadIoError(IoError),
    NotificationReadProcessingError,
    NotificationReadMissingDeviceToken,
    NotificationReadMissingTopic,
    NotificationReadMissingPayload,
    NotificationReadMissingIdentifier,
    NotificationReadMissingExpiration,
    NotificationReadMissingPriority,
    NotificationReadInvalidTokenSize,
    NotificationReadInvalidTopicSize,
    NotificationReadInvalidToken,
    NotificationReadShutdown,
    NotificationReadUnknown,
}

pub struct NotificationReader<'a, T: 'a> {
    //reader: &'a mut BufferedReader<T>,
    reader: &'a mut T,
    current_frame_position: int,
    current_device_token: Option<String>,
    current_payload: Option<Json>,
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

    fn read_header(&mut self) -> Result<APNSHeader, IoError> {
        let command = try!(self.reader.read_byte());
        let frame_length = try!(self.reader.read_be_u32());
        let header = APNSHeader{command: command, frame_length: frame_length};
        return Ok(header);
    }

    fn read_item_length(&mut self) -> Result<uint, IoError> {
        let result = try!(self.reader.read_be_u16());
        self.current_frame_position += 2;
        return Ok(from_u16(result).unwrap());
    }

    fn read_device_token(&mut self) -> Result<(),IoError> {
        let item_length = try!(self.read_item_length());
        let raw = U8Vec(try!(self.reader.read_exact(item_length)));
        let device_token = raw.to_hex();
        self.current_device_token = Some(device_token.clone());
        debug!("Read device token {0}", device_token.clone());
        self.current_frame_position += item_length as int;
        return Ok(());
    }

    fn read_identifier(&mut self) -> Result<(), IoError> {
        try!(self.read_item_length());
        self.current_identifier = Some(try!(self.reader.read_be_u32()));
        debug!("Read identifier {0}", self.current_identifier.unwrap());
        self.current_frame_position += 4;
        return Ok(());
    }

    fn read_expiration(&mut self) -> Result<(), IoError> {
        try!(self.read_item_length());
        self.current_expiration_date = Some(try!(self.reader.read_be_u32()));
        debug!("Read expiration date {0}", self.current_expiration_date.unwrap());
        self.current_frame_position += 4;
        return Ok(());
    }

    fn read_priority(&mut self) -> Result<(), IoError> {
        try!(self.read_item_length());
        self.current_priority = Some(try!(self.reader.read_u8()));
        debug!("Read priority {0}", self.current_priority.unwrap());
        self.current_frame_position += 1;
        return Ok(());
    }

    fn read_payload(&mut self) -> Result<(), IoError> {
        let item_length = try!(self.read_item_length());
        let mut payload: Vec<u8>  = Vec::with_capacity(item_length as uint);
        for _ in range(0, item_length) {
            payload.push(try!(self.reader.read_u8()));
        }
        let payload_str = String::from_utf8(payload.to_vec()).unwrap();
        debug!("Read payload {0}", payload_str);
        let decoded: Option<Json> = from_str(payload_str.as_slice());
        match decoded {
            Some(parsed_json) => self.current_payload = Some(parsed_json),
            None => self.current_payload = None
        }
        self.current_frame_position += item_length as int;
        return Ok(());
    }

    fn reset(&mut self) {
        self.current_device_token = None;
        self.current_payload = None;
        self.current_identifier = None;
        self.current_expiration_date = None;
        self.current_priority = None;
    }

    fn assert_notification(&self) -> Result<Notification, NotificationReadError> {
        if self.current_device_token.is_none(){
            return Err(NotificationReadMissingDeviceToken);
        }
        if self.current_payload.is_none(){
            return Err(NotificationReadMissingPayload);
        }
        if self.current_identifier.is_none(){
            return Err(NotificationReadMissingIdentifier);
        }
        if self.current_expiration_date.is_none(){
            return Err(NotificationReadMissingExpiration);
        }
        if self.current_priority.is_none() {
            return Err(NotificationReadMissingPriority);
        }
        return Ok(Notification {
            device_token: self.current_device_token.clone().unwrap(),
            payload: self.current_payload.clone().unwrap(),
            identifier: self.current_identifier.unwrap(),
            expiration_date: self.current_expiration_date.unwrap(),
            priority: self.current_priority.unwrap(),
        });
    }

    pub fn read_notification(&mut self) -> Result<Notification, NotificationReadError> {
        self.reset();
        self.current_frame_position = 0;
        let header:APNSHeader = match self.read_header() {
            Ok(header) => header,
            Err(io_error) => return Err(NotificationReadIoError(io_error))
        };
        info!("Read header, command: {0}, data length: {1}", header.command, header.frame_length);

        while self.current_frame_position < (header.frame_length - 1) as int {
            let item_id = match self.reader.read_u8() {
                Ok(id) => id,
                Err(io_error) => return Err(NotificationReadIoError(io_error))
            };
            self.current_frame_position += 1;

            let read_result = match item_id {
                1u8 => self.read_device_token(),
                2u8 => self.read_payload(),
                3u8 => self.read_identifier(),
                4u8 => self.read_expiration(),
                5u8 => self.read_priority(),
                other_id => fail!("Unknown item id {}", other_id)
            };
            match read_result {
                Ok(_) =>{},
                Err(io_error) =>
                    return Err(NotificationReadIoError(io_error))
            };
        }
        return self.assert_notification();
    }
}

