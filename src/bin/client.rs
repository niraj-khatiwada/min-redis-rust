use bytes::Bytes;
use mini_redis::{client, Result};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

type Responder<T> = oneshot::Sender<Result<T>>;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<Command>(32);

    let tx1 = tx.clone();
    let tx2 = tx.clone();

    let task1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        tx1.send(Command::Set {
            key: String::from("ping"),
            val: "pong".into(),
            resp: resp_tx,
        })
        .await
        .unwrap();

        let rs = resp_rx.await.unwrap();
        println!("[task1] GOT {:?}", rs);
    });

    let task2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        tx2.send(Command::Get {
            key: String::from("ping"),
            resp: resp_tx,
        })
        .await
        .unwrap();

        let rs = resp_rx.await.unwrap();
        println!("[task2] GOT {:?}", rs);
    });

    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:16379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, resp } => {
                    let res = client.get(&key).await.unwrap();
                    let _ = resp.send(Ok(res));
                }
                Command::Set { key, val, resp } => {
                    client.set(&key, val).await.unwrap();
                    let _ = resp.send(Ok(()));
                }
            }
        }
    });

    task1.await.unwrap();
    task2.await.unwrap();
    manager.await.unwrap();

    Ok(())
}
