use std::{net::SocketAddr, sync::Arc};

use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::WriteHalf, TcpListener},
    sync::{
        broadcast::{self, Receiver, Sender},
        Mutex,
    },
};

type Tx = Sender<(String, SocketAddr)>;
type Rx = Receiver<(String, SocketAddr)>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    // Create a broadcast channel to receive messages from the server.
    let broadcast: (Tx, Rx) = broadcast::channel(10);
    let tx: Tx = broadcast.0;
    let _rx: Rx = broadcast.1;

    loop {
        let mut history = String::new();
        // Listen for incoming connections.
        let (mut socket, addr) = listener.accept().await.ok().unwrap();

        let tx: Tx = tx.clone();
        let mut rx: Rx = tx.subscribe();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            let mut usr = addr.to_string();

            writer.write(b"welcome to the server!\n").await.unwrap();
            loop {
                // creates two virtual threads, one for sending and one for receiving
                tokio::select! {
                    _result = reader.read_line(&mut line) => {

                        prefix_checker(&tx, addr, &mut writer, line.clone(), history.clone(), &mut usr).await;
                        shift_history(&mut history, line.clone());
                        history.push_str(&line);
                        line.clear();
                    }

                    result = rx.recv() => {
                        let (msg, recv_addr) = result.unwrap();
                        if msg.len() > 2 && addr != recv_addr {
                            writer.write_all((msg.as_str()).as_bytes()).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}

fn shift_history<'b>(history: &mut String, new_element: String) {
    history.push_str(&new_element);
}

async fn prefix_checker<'a>(
    tx: &Tx,
    addr: SocketAddr,
    writer: &mut WriteHalf<'a>,
    line: String,
    history: String,
    usr: &mut String,
) {
    if line.as_bytes()[0] == b':' {
        let command = line.split_whitespace().collect::<Vec<&str>>();
        match command[0].trim() {
            ":quit" => {
                writer.write(b"bye\n").await.unwrap();
            }
            ":history" => {
                writer.write(b"*chat history* \n").await.unwrap();
                writer.write(history.as_bytes()).await.unwrap();
            }
            ":username" => {
                if command.len() == 2 {
                    *usr = command[1].to_string();
                } else {
                    writer.write(b"invalid username\n").await.unwrap();
                }
            }
            &_ => {
                writer.write(b"unknown command\n").await.unwrap();
            }
        }
    } else {
        tx.send((usr.to_owned() + ": " + &line.clone(), addr))
            .unwrap();
    }
}
