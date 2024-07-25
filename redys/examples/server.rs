use bytes::Bytes;
use mini_redis::{
    Command::{self, Get, Set},
    Connection, Frame,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
#[tokio::main]
async fn main() {
    // Bind the listener to the address
    let listener: TcpListener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db: Arc<Mutex<HashMap<String, Bytes>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await.unwrap();
        let dbx: Arc<Mutex<HashMap<String, Bytes>>> = db.clone();
        tokio::spawn(async move {
            let x = process(socket, dbx);
            x.await;
        });
    }
}

async fn process(socket: TcpStream, db: Db) {
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let resp = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db: MutexGuard<HashMap<String, Bytes>> = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                println!(
                    "Setting `{}`: `{}`",
                    cmd.key().to_string(),
                    String::from_utf8(cmd.value().to_vec()).unwrap()
                );
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                match db.get(cmd.key()) {
                    Some(value) => {
                        println!(
                            "Getting `{}`: `{:?}`",
                            cmd.key().to_string(),
                            String::from_utf8(value.clone().to_vec()),
                        );
                        // `Frame::Bulk` expects data to be of type `Bytes`. This
                        // type will be covered later in the tutorial. For now,
                        // `&Vec<u8>` is converted to `Bytes` using `into()`.
                        Frame::Bulk(value.clone().into())
                    }
                    None => {Frame::Null},
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };
        connection.write_frame(&resp).await.unwrap();
    }
}
