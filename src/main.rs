#![feature(phase)]

extern crate http;
extern crate openssl;
extern crate serialize;
extern crate time;
extern crate getopts;
#[phase(plugin, link)] extern crate log;

use http::server::{Server};
use std::io::net::ip::{SocketAddr, IpAddr};
use getopts::{reqopt, optopt, getopts, usage};
use std::os;

mod notifications;
mod stateholder;
mod apns_server;
mod notification_http_server;




fn main(){
    let args: Vec<String> = os::args();
    let program = args[0].clone();

    let opts = [
        optopt("h", "anps-host", "The ip address the apns server will be available on, default 127.0.0.1", "APNS SERVER"),
        optopt("p", "apns-port", "The port the apns server will be available on, default 9123", "APNS PORT"),
        optopt("n", "notification-server-ip", "The ip address the notification server http interface will bind to, default 127.0.0.1", "HTTP SERVER"),
        optopt("N", "notification-server-port", "The port the notification server http interface will bind to, default 8080", "HTTP PORT"),
        reqopt("", "cert-path", "Path to the ssl certificate to use", "SSL CERT"),
        reqopt("", "private-key-path", "Path to the ssl private key to use", "SSL PRIVATE KEY"),
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => { fail!(usage("Usage: ", opts))}
    };

    let apns_ip: IpAddr = from_str(matches.opt_str("h").unwrap_or(String::from_str("127.0.0.1")).as_slice()).expect(
        "Invalid ip specified for apns server ip");

    let apns_port: u16 = from_str(matches.opt_str("p").unwrap_or(String::from_str("9123")).as_slice()).expect(
            "invalid port specified for apns server");

    let http_ip: IpAddr = from_str(matches.opt_str("n").unwrap_or(String::from_str("127.0.0.1")).as_slice()).expect(
        "invalid ip specified for http notification server");

    let http_port: u16 = from_str(matches.opt_str("N").unwrap_or(String::from_str("8080")).as_slice()).expect(
        "invalid port specified for http notification server");

    let ssl_cert_path = Path::new(matches.opt_str("cert-path").expect("No cert path set"));
    let ssl_private_key_path = Path::new(matches.opt_str("private-key-path").expect("no private key path set"));

    let (mut state_holder, stateholder_interface) = stateholder::StateHolder::new();
    let stateholder_proc = spawn(proc() {
        state_holder.start();
    });

    let http_stateholder_interface = stateholder_interface.clone();
    let http_server_addr = SocketAddr{ip: http_ip, port: http_port};
    let server_proc = spawn(proc() {
        let server = notification_http_server::NotificationHttpServer::new(http_stateholder_interface, http_server_addr);
        server.serve_forever();
        format!("HTTP server failed for some reason, see previous logs");
    });

    let addr: SocketAddr = SocketAddr{ip: apns_ip, port: apns_port};
    let apns_server = apns_server::APNSServer::new(addr, stateholder_interface.clone(), ssl_cert_path, ssl_private_key_path);
    let result = apns_server.start();
    if result.is_err(){
        fail!("failed to bind to apns server address");
    }
}
