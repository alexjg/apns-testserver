#![feature(phase)]

extern crate http;
extern crate openssl;
extern crate serialize;
extern crate time;
#[phase(plugin, link)] extern crate log;

use http::server::{Server};
use std::io::net::ip::{SocketAddr};

mod notifications;
mod stateholder;
mod apns_server;
mod notification_http_server;


fn main(){

    let (mut state_holder, stateholder_interface) = stateholder::StateHolder::new();
    let stateholder_proc = spawn(proc() {
        state_holder.start();
    });

    let http_stateholder_interface = stateholder_interface.clone();
    let server_proc = spawn(proc() {
        let server = notification_http_server::NotificationHttpServer::new(http_stateholder_interface);
        server.serve_forever();
    });

    let addr: SocketAddr = from_str("127.0.0.1:9123").unwrap();
    let ssl_cert_path = Path::new("/home/alex/temp/selfcert/server.crt");
    let ssl_private_key_path = Path::new("/home/alex/temp/selfcert/server.key");
    let apns_server = apns_server::APNSServer::new(addr, stateholder_interface.clone(), ssl_cert_path, ssl_private_key_path);
    apns_server.start();
}
