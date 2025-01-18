use bytes::Bytes;
use dashmap::DashMap;
use mini_redis::{Command, Connection, Frame};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

type DB = Arc<Mutex<DashMap<String, Bytes>>>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:16379").await.unwrap();
    let db: DB = Arc::new(Mutex::new(DashMap::new()));

    loop {
        let db = db.clone();
        let (socket, addr) = listener.accept().await.unwrap();
        println!("New connection: {}", addr);
        tokio::spawn(async {
            process(socket, db).await;
        });
    }
}

async fn process(socket: TcpStream, db: DB) {
    let mut conn = Connection::new(socket);

    while let Some(frame) = conn.read_frame().await.unwrap() {
        println!("FRAME {:?}", frame);
        let rs = match Command::from_frame(frame).unwrap() {
            Command::Get(cmd) => {
                let db_lock = db.lock();
                let get = db_lock.get(cmd.key());
                match get {
                    Some(val) => Frame::Bulk(val.clone()),
                    None => Frame::Null,
                }
            }
            Command::Set(cmd) => {
                let db_lock = db.lock();
                db_lock.insert(String::from(cmd.key()), cmd.value().clone());
                Frame::Simple(String::from("OK"))
            }
            Command::Unknown(cmd) => {
                panic!("cmd not found {:?}", cmd);
            }
            _ => panic!("cmd not found"),
        };
        conn.write_frame(&rs).await.unwrap();
    }
}
