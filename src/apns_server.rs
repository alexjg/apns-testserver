use std::io::net::ip::{SocketAddr};
use std::io::{TcpListener, TcpStream};
use std::io::{Listener, Acceptor, IoError, IoErrorKind, EndOfFile};
use stateholder::StateHolderInterface;
use notifications;
use serialize::json;
use openssl::ssl;

pub struct APNSServer {
    address: SocketAddr,
    ssl_cert_path: Path,
    ssl_private_key_path: Path,
    stateholder_interface: StateHolderInterface,
}

impl APNSServer {
    pub fn new(address: SocketAddr, stateholder_interface: StateHolderInterface, ssl_cert_path: Path, ssl_private_key_path: Path) -> APNSServer {
        APNSServer{
            address: address,
            stateholder_interface: stateholder_interface,
            ssl_cert_path: ssl_cert_path,
            ssl_private_key_path: ssl_private_key_path,
        }
    }

    pub fn start(&self) -> Result<(), IoError> {
        let listener = try!(TcpListener::bind(format!("{}", self.address.ip).as_slice(), self.address.port));
        let mut acceptor = listener.listen();
        println!("Listening started, ready to accept");
        for opt_stream in acceptor.incoming(){
            let state_holder_interface = self.stateholder_interface.clone();
            spawn(proc() {
                info!("Receiving frame");
                let mut stream = opt_stream.unwrap();
                let mut ssl_context = ssl::SslContext::new(ssl::Sslv3).unwrap();
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
        return Ok(());
    }
}
