#![feature(phase)]

extern crate http;
extern crate openssl;
extern crate serialize;
extern crate time;
#[phase(plugin, link)] extern crate log;

use http::server::{Config, Server, Request, ResponseWriter};
use http::headers::content_type::MediaType;
use http::method::{Delete};
use openssl::{ssl};
use serialize::{json};
use serialize::json::{ToJson};
use std::comm::{Sender, Receiver};
use std::io::{Listener, Acceptor, IoError, IoErrorKind, EndOfFile};
use std::path;
use std::io::net::tcp::{TcpListener, TcpStream};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::vec::{Vec};

mod notifications;
mod stateholder;


#[deriving(Clone)]
struct NotificationHttpServer{
    stateholder_interface: stateholder::StateHolderInterface,
}

impl NotificationHttpServer {
    pub fn new(state_holder_channel: Sender<stateholder::StateHolderCommand>) -> NotificationHttpServer {
        NotificationHttpServer{
            stateholder_interface: stateholder::StateHolderInterface::new(state_holder_channel)
        }
    }
}

impl Server for NotificationHttpServer {

    fn get_config(&self) -> Config {
        Config {bind_address: SocketAddr{ip: Ipv4Addr(127, 0, 0, 1), port: 8081}}
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


fn main(){

    let (state_holder_channel, state_holder_port) = channel::<stateholder::StateHolderCommand>();
    let state_holder_proc = spawn(proc() {
        let mut state_holder = stateholder::StateHolder::new(state_holder_port);
        state_holder.start();
    });


    let server_stateholder_channel = state_holder_channel.clone();
    let server_proc = spawn(proc() {
        //let server = NotificationHttpServer::new(notifications);
        let server = NotificationHttpServer::new(server_stateholder_channel);
        server.serve_forever();
    });

    let mut acceptor = TcpListener::bind("127.0.0.1", 9123).listen().unwrap();
    println!("Listening started, ready to accept");
    for opt_stream in acceptor.incoming(){
        let state_holder_interface = stateholder::StateHolderInterface::new(state_holder_channel.clone());
        spawn(proc() {
            info!("Receiving frame");
            let mut stream = opt_stream.unwrap();
            let mut ssl_context = ssl::SslContext::new(ssl::Sslv3).unwrap();
            let cert_path = path::Path::new("/home/alex/temp/selfcert/server.crt");
            ssl_context.set_certificate_file("/home/alex/temp/selfcert/server.crt", ssl::PEM);
            ssl_context.set_verify(ssl::SslVerifyNone, None);
            ssl_context.set_private_key_file("/home/alex/temp/selfcert/server.key", ssl::PEM);
            let ssl = ssl::Ssl::new(&ssl_context).unwrap();
            let mut ssl_stream = ssl::SslStream::new_server_from(ssl, stream).unwrap();
            loop {
                let mut reader: notifications::NotificationReader<ssl::SslStream<TcpStream>> = notifications::NotificationReader::new(&mut ssl_stream);
                match reader.read_notification(){
                    Ok(notification) => {
                        info!("Read notification {}", json::encode(&notification));
                        state_holder_interface.add_notification(notification);
                    }
                    Err(notifications::NotificationReadIoError(io_error)) => match io_error.kind {
                        EndOfFile => {
                            info!("End of input, closing");
                            break;
                        },
                        _ => {
                            info!("Unknown error while reading notificaiton, closing channel");
                            break;
                        }
                    },
                    _ => {
                        info!("Unknown error while reading notificaiton, closing channel");
                        break;
                    }
                }
            }
        })
    }

}
