extern crate config;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate websocket;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use websocket::client::ClientBuilder;
use websocket::stream::sync::TlsStream;
use websocket::sync::Client;
use websocket::OwnedMessage;

type ThreadId = u32;
type ThreadIdTx = Sender<ThreadId>;
type ThreadIdRx = Receiver<ThreadId>;
type MessageTx = Sender<String>;
type MessageRx = Receiver<String>;
type ClientTxs = HashMap<ThreadId, MessageTx>;
type AM<T> = Arc<Mutex<T>>;

const ADDR: &str = "0xb6ed7644c69416d67b522e20bc294a9a9b405b31";
const SIG_MT: &str = "0x8a769d35"; // miningTarget()
const SIG_CN: &str = "0x8ae0368b"; // challengeNumber()
const TOP_MT: &str = "0xcf6fbb9dcea7d07263ab4f5c3a92f53af33dffc421d9d121e1c74b307e68189d"; // Mint()

fn main() {
    let mut settings = config::Config::default();
    if let Err(_) = settings.merge(config::File::with_name("config")) {
        println!("ERROR: can't find config.toml");
        process::exit(1);
    }
    let wss_url = settings.get_str("provider_url").expect("bad config");
    let https_url = settings.get_str("provider_url_https").expect("bad config");
    let listen_ap = settings.get_str("listen").expect("bad config");

    let (tid_tx, tid_rx) = channel::<ThreadId>();
    let client_txs = Arc::new(Mutex::new(ClientTxs::new()));
    let mining_target = Arc::new(Mutex::new(String::new()));
    let challenge_number = Arc::new(Mutex::new(String::new()));

    start_stats_thread(client_txs.clone(), tid_rx);
    start_ping_thread(client_txs.clone());
    start_params_thread(
        https_url,
        client_txs.clone(),
        mining_target.clone(),
        challenge_number.clone(),
    );
    start_stream_thread(
        wss_url,
        client_txs.clone(),
        mining_target.clone(),
        challenge_number.clone(),
    );
    serve_forever(
        listen_ap,
        tid_tx,
        client_txs.clone(),
        mining_target.clone(),
        challenge_number.clone(),
    );
}

fn handle_client(mut stream: TcpStream, tid_tx: ThreadIdTx, tid: u32, mrx: MessageRx, msg: String) {
    let _ = stream.set_nonblocking(true);
    if msg != "" {
        stream.write((msg + "\n").as_bytes()).ok();
    }
    loop {
        let still_connected = if let Ok(msg) = mrx.recv_timeout(Duration::from_secs(1)) {
            // another thread gave us data to send to client
            stream.write((msg + "\n").as_bytes()).is_ok()
        } else {
            // check tcp stream about once per second
            match stream.read(&mut [0; 1024]) {
                // T: got data (ignore); F: disconnected
                Ok(len) => len > 0,
                // T: no data (normal); F: disconnected
                Err(e) => e.kind() == WouldBlock,
            }
        };
        if !still_connected {
            break;
        }
    }
    if let Ok(addr) = stream.peer_addr() {
        println!("[#{}] killing connection to {}", tid, addr);
    }
    stream.shutdown(Shutdown::Both).ok();
    println!("[#{}] client thread done", tid);
    tid_tx.send(tid).ok();
}

fn start_stats_thread(client_txs_m: AM<ClientTxs>, tid_rx: ThreadIdRx) {
    thread::spawn(move || loop {
        if let Ok(msg) = tid_rx.recv() {
            let mut client_txs = client_txs_m.lock().unwrap();
            client_txs.remove(&msg);
            println!("thread {} removed ({} threads)", msg, client_txs.len());
        }
    });
}

fn start_ping_thread(client_txs_m: AM<ClientTxs>) {
    thread::spawn(move || {
        let ping = json!({"ping": "ping"}).to_string();
        loop {
            for (_, mtx) in client_txs_m.lock().unwrap().iter() {
                let _ = mtx.send(ping.clone());
            }
            thread::sleep(Duration::from_secs(30));
        }
    });
}

#[derive(Serialize, Deserialize, Debug)]
struct BatchReply {
    id: u32,
    result: String,
}

fn start_params_thread(
    https_url: String,
    client_txs_m: AM<ClientTxs>,
    mining_target_m: AM<String>,
    challenge_number_m: AM<String>,
) {
    let batch_req = json!(
        [
            {
                "method": "eth_call",
                "params": [{"to": ADDR, "data": SIG_MT}, "latest"],
                "id": 1,
                "jsonrpc": "2.0"
            },
            {
                "method": "eth_call",
                "params": [{"to": ADDR, "data": SIG_CN}, "latest"],
                "id": 2,
                "jsonrpc": "2.0"
            }
        ]
    );
    thread::spawn(move || loop {
        let client = reqwest::Client::new();
        let r = client.post(&https_url).body(batch_req.to_string()).send();
        if let Some(mut body) = r.ok() {
            let json = body.text().unwrap();
            let r = serde_json::from_str::<Vec<BatchReply>>(&json).ok();
            if let Some(replies) = r {
                let mut mt = mining_target_m.lock().unwrap();
                let mut cn = challenge_number_m.lock().unwrap();
                let mut changed = false;
                for reply in replies.iter() {
                    if reply.id == 1 {
                        changed = changed || (mt.to_string() != reply.result);
                        mt.clear();
                        mt.push_str(&reply.result);
                    } else {
                        changed = changed || (cn.to_string() != reply.result);
                        cn.clear();
                        cn.push_str(&reply.result);
                    }
                }
                if changed {
                    println!("CHANGED!!!");
                    broadcast_mining_params(client_txs_m.clone(), mt.to_string(), cn.to_string());
                }
            }
        }
        thread::sleep(Duration::from_secs(10));
    });
}

fn broadcast_mining_params(client_txs_m: AM<ClientTxs>, mt: String, cn: String) {
    let client_txs = client_txs_m.lock().unwrap();
    for (_, mtx) in client_txs.iter() {
        let json = json!({
            "miningTarget": mt,
            "challengeNumber": cn
        })
        .to_string();
        mtx.send(json).ok();
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    params: Params,
}
#[derive(Serialize, Deserialize, Debug)]
struct Params {
    result: Result,
}
#[derive(Serialize, Deserialize, Debug)]
struct Result {
    data: String,
}

fn start_stream_thread(
    wss_url: String,
    client_txs_m: AM<ClientTxs>,
    mining_target_m: AM<String>,
    challenge_number_m: AM<String>,
) {
    thread::spawn(move || loop {
        let ct_m = client_txs_m.clone();
        let mt_m = mining_target_m.clone();
        let cn_m = challenge_number_m.clone();
        if let Some(mut cb) = ClientBuilder::new(&wss_url).ok() {
            if let Ok(client) = cb.connect_secure(None) {
                handle_stream(client, ct_m, mt_m, cn_m);
            }
        }
        thread::sleep(Duration::from_secs(1));
    });
}

fn handle_stream(
    mut client: Client<TlsStream<TcpStream>>,
    client_txs_m: AM<ClientTxs>,
    mining_target_m: AM<String>,
    challenge_number_m: AM<String>,
) {
    println!("wss connected.");
    let req = OwnedMessage::Text(
        json!(
        {
            "method": "eth_subscribe",
            "params": ["logs", {"address": ADDR, "topics": [TOP_MT]}],
            "id": 1,
            "jsonrpc": "2.0"
        }
        )
        .to_string(),
    );
    client.send_message(&req).ok();
    for message in client.incoming_messages() {
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                println!("wss error {:?}", e);
                return;
            }
        };
        match message {
            OwnedMessage::Close(_) => {
                println!("wss closed");
                return;
            }
            OwnedMessage::Ping(data) => {
                println!("wss ping {:?}", data);
            }
            _ => {
                println!("wss message: {:?}", message);
                if let OwnedMessage::Text(ref txt) = message {
                    let json = txt.to_string();
                    let r = serde_json::from_str::<Message>(&json).ok();
                    if let Some(message) = r {
                        let data = message.params.result.data;
                        if data.len() == 194 {
                            let mt = mining_target_m.lock().unwrap();
                            let mut cn = challenge_number_m.lock().unwrap();
                            cn.clear();
                            cn.push_str("0x");
                            cn.push_str(&data[130..194]);
                            broadcast_mining_params(
                                client_txs_m.clone(),
                                mt.to_string(),
                                cn.to_string(),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn serve_forever(
    listen_ap: String,
    tid_tx: ThreadIdTx,
    ct_m: AM<ClientTxs>,
    mt_m: AM<String>,
    cn_m: AM<String>,
) {
    let listener = TcpListener::bind(listen_ap.clone()).unwrap();
    println!("Server listening on {}", listen_ap);
    let mut next_id = 0;
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let tid = next_id;
            start_client_thread(
                stream,
                tid,
                tid_tx.clone(),
                mt_m.clone(),
                cn_m.clone(),
                ct_m.clone(),
            );
            next_id += 1;
        }
    }
    drop(listener);
}

fn start_client_thread(
    stream: TcpStream,
    tid: u32,
    tid_tx: ThreadIdTx,
    mt_m: AM<String>,
    cn_m: AM<String>,
    ct_m: AM<ClientTxs>,
) {
    let tid_tx2 = tid_tx.clone();
    let (mtx, mrx) = channel();
    if let Ok(addr) = stream.peer_addr() {
        println!("new connection: {}", addr);
        let mt: String = mt_m.lock().unwrap().to_string();
        let cn: String = cn_m.lock().unwrap().to_string();
        let msg: String = if mt != "" && cn != "" {
            json!({
                "miningTarget": mt,
                "challengeNumber": cn
            })
            .to_string()
        } else {
            "".to_string()
        };
        ct_m.lock().unwrap().insert(tid, mtx);
        let num = ct_m.lock().unwrap().len();
        println!("{} threads", num);
        thread::spawn(move || handle_client(stream, tid_tx2, tid, mrx, msg));
    }
}
