#![feature(phase)]

extern crate serialize;
extern crate http;
extern crate time;
#[phase(plugin, link)] extern crate log;

use http::server::{Config, Server, Request, ResponseWriter};
use http::headers::content_type::MediaType;
use serialize::{json};
use std::comm::{Sender, Receiver};
use std::io::{Listener, Acceptor};
use std::io::net::tcp::{TcpListener, TcpStream};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::vec::{Vec};

mod notifications;

#[deriving(Clone)]
enum StateHolderCommand{
    SetState(Vec<notifications::Notification>),
    Append(notifications::Notification),
    GetState,
    ClearState,
}

#[deriving(Clone)]
enum StateHolderResponse{
    Ok(Vec<notifications::Notification>),
}

struct StateStore{
    notifications: Vec<notifications::Notification>,
}

impl StateStore {
    fn clear(&mut self) {
        self.notifications.clear()
    }
    fn reset(&mut self, new_notifications: &mut Vec<notifications::Notification>) -> Vec<notifications::Notification> {
        self.clear();
        self.notifications.extend(new_notifications.clone().into_iter());
        return self.notifications.clone();
    }
    fn get(&self) -> Vec<notifications::Notification>{
        return self.notifications.clone();
    }
    fn append(&mut self, notification: notifications::Notification) -> Vec<notifications::Notification> {
        info!("Adding notification to state");
        self.notifications.push(notification.clone());
        return self.notifications.clone()
    }
}


#[deriving(Clone)]
struct NotificationHttpServer<'a> {
    tx_stateholder: Sender<StateHolderCommand>,
    rx_stateholder: &'a Receiver<StateHolderResponse>,
}

impl<'a> NotificationHttpServer<'a> {
    pub fn new(tx_command: Sender<StateHolderCommand>, rx_response: Receiver<StateHolderResponse>) {
        NotificationHttpServer{
            tx_stateholder: tx_command,
            rx_stateholder: rx_response,
        }
    }
}

impl<'a> Server for NotificationHttpServer<'a> {

    fn get_config(&self) -> Config {
        Config {bind_address: SocketAddr{ip: Ipv4Addr(127, 0, 0, 1), port: 8081}}
    }

    fn handle_request(&self, _r:Request, w: &mut ResponseWriter) {
        w.headers.date = Some(time::now_utc());
        w.headers.content_length = Some(14);
        w.headers.content_type = Some(MediaType {
            type_: String::from_str("text"),
            subtype: String::from_str("html"),
            parameters: vec!((String::from_str("charset"), String::from_str("UTF-8")))
        });
        w.headers.server = Some(String::from_str("Example"));
        self.tx_stateholder.send(GetState);
        let notifications = match self.rx_stateholder.recv() {
            Ok(notifications) => notifications,
            _ => fail!("Unknown response from staeholder")
        };
        w.write(json::encode(&notifications).into_bytes().as_slice())
    }

}


fn main(){

    let (tx_state, rx_state) = channel::<StateHolderCommand>();
    let (tx_listener, rx_listener) = channel::<StateHolderResponse>();

    let state_holder  = spawn(proc() {
        let mut notifications = Vec::new();
        let mut state = StateStore{notifications: notifications};
        loop {
            let response: StateHolderResponse = match rx_state.recv() {
                SetState(new_notifications) => Ok(state.reset(new_notifications)),
                Append(new_notification) => Ok(state.append(new_notification)),
                GetState => Ok(state.get()),
                _ => fail!("Not implemented yet")
            };
            tx_listener.send(response);
        }
    });

    let http_tx = tx_state.clone();
    let http_rx = rx_listener.clone();
    let server_proc = spawn(proc() {
        let server = NotificationHttpServer::new(http_tx, http_rx);
        server.serve_forever();
    });

    let mut acceptor = TcpListener::bind("127.0.0.1", 9123).listen().unwrap();
    println!("Listening started, ready to accept");
    for opt_stream in acceptor.incoming(){
        let acceptor_state_tx = tx_state.clone();
        spawn(proc() {
            info!("Receiving frame");
            let mut stream = opt_stream.unwrap();
            loop {
                let mut reader: notifications::NotificationReader<TcpStream> = notifications::NotificationReader::new(&mut stream);
                let notification: notifications::Notification = reader.read_notification().unwrap();
                info!("Read notification {}", json::encode(&notification));
                acceptor_state_tx.send(Append(notification));
            }
        })
    }

}
