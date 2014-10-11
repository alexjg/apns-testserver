use http::server::{Config, Server, Request, ResponseWriter};
use http::headers::content_type::MediaType;
use http::method::{Delete};
use serialize::{json};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use time;

use notifications;
use stateholder;

#[deriving(Clone)]
pub struct NotificationHttpServer{
    stateholder_interface: stateholder::StateHolderInterface,
    address: SocketAddr,
}

impl NotificationHttpServer {
    pub fn new(stateholder_interface: stateholder::StateHolderInterface, address: SocketAddr) -> NotificationHttpServer {
        NotificationHttpServer{
            stateholder_interface: stateholder_interface,
            address: address,
        }
    }
}

impl Server for NotificationHttpServer {

    fn get_config(&self) -> Config {
        Config {bind_address: self.address}
    }

    fn handle_request(&self, request :Request, w: &mut ResponseWriter) {

        w.headers.date = Some(time::now_utc());
        w.headers.content_type = Some(MediaType {
            type_: String::from_str("application"),
            subtype: String::from_str("json"),
            parameters: vec!((String::from_str("charset"), String::from_str("UTF-8")))
        });
        w.headers.server = Some(String::from_str("Example"));
        w.write_headers();
        match request.method {
            Delete => {
                self.stateholder_interface.clear();
                let message = "{\"status\": \"OK\"}";
                w.headers.content_length = Some(message.len());
                w.write_line(message).unwrap();
            },
            _ => {
                let notifications = self.stateholder_interface.get_state();
                let as_json = json::encode(&notifications);
                w.headers.content_length = Some(as_json.len());
                info!("returning {0}", as_json);
                w.write_line(as_json.as_slice()).unwrap();
            }
        };
    }

}
