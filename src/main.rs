#![feature(write_all_vectored)]
#![feature(io_slice_advance)]
#![feature(io_error_other)]

use num_enum::TryFromPrimitive;
use tokio::io::BufStream;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

#[repr(i32)]
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
enum Type {
    Login = 3,
    Command = 2,
    MultiPacketResponse = 0,
}

pub struct Packet {
    request_id: i32,
    r#type: Type,
    payload: Vec<u8>,
}

pub struct Client {
    conn: BufStream<TcpStream>,
}

impl Client {
    pub async fn new<A: ToSocketAddrs>(addr: A) -> tokio::io::Result<Self> {
        Ok(Self {
            conn: BufStream::new(TcpStream::connect(addr).await?),
        })
    }

    // TODO: use [write_all_vectored] when it's ready
    pub async fn send(&mut self, packet: &Packet) -> tokio::io::Result<()> {
        // + 1 for the null byte
        let length = 2 * std::mem::size_of::<i32>() + packet.payload.len() + 1;

        self.conn.write_all(&(length as i32).to_le_bytes()).await?;
        self.conn
            .write_all(&packet.request_id.to_le_bytes())
            .await?;
        self.conn
            .write_all(&(packet.r#type as i32).to_le_bytes())
            .await?;
        self.conn.write_all(&packet.payload).await?;
        self.conn.write_all(b"\0").await?;

        Ok(())
    }

    pub async fn recv<'a>(&'a mut self) -> tokio::io::Result<Packet> {
        // Look into sync/stream
        // mcrcon spec says you can have at most 4098 bytes

        let length = self.conn.read_i32_le().await?;
        let request_id = self.conn.read_i32_le().await?;
        let r#type: Type = self
            .conn
            .read_i32_le()
            .await?
            .try_into()
            .map_err(tokio::io::Error::other)?;

        let mut payload = Vec::with_capacity(
            length as usize - std::mem::size_of_val(&request_id) - std::mem::size_of_val(&r#type),
        );

        self.conn.read_until(0u8, &mut payload).await?;

        Ok(Packet {
            request_id,
            r#type,
            payload,
        })
    }

    // pub fn login(&self, _password: String) -> std::io::Result<Self> {
    //     self.conn.read_a
    // }
}

fn main() {
    println!("Hello, world!");
}
