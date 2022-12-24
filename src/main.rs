#![feature(write_all_vectored)]
#![feature(io_error_other)]

use num_enum::TryFromPrimitive;
use std::collections::HashMap;
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

#[derive(Debug)]
pub struct Packet {
    request_id: i32,
    r#type: Type,
    payload: Vec<u8>,
}

impl Packet {
    fn new<T: Into<Vec<u8>>>(request_id: i32, r#type: Type, payload: T) -> Self {
        Self {
            request_id,
            r#type,
            payload: payload.into(),
        }
    }

    pub fn login(request_id: i32, password: String) -> Self {
        Self::new(request_id, Type::Login, password)
    }
}

pub struct Client {
    conn: BufStream<TcpStream>,
    current_request_id: i32,
    responses: HashMap<i32, Packet>,
}

impl Client {
    pub async fn new<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        Ok(Self {
            conn: BufStream::new(TcpStream::connect(addr).await?),
            current_request_id: 0,
            responses: HashMap::new(),
        })
    }

    pub async fn send(&mut self, packet: Packet) -> std::io::Result<Packet> {
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

        self.conn.flush().await?;

        loop {
            match self.responses.remove(&packet.request_id) {
                Some(packet) => return Ok(packet),
                None => self.recv().await?,
            }
        }
    }

    pub async fn recv(&mut self) -> std::io::Result<()> {
        // Look into sync/stream
        // mcrcon spec says you can have at most 4098 bytes

        let length = self.conn.read_i32_le().await?;
        let request_id = self.conn.read_i32_le().await?;
        let r#type: Type = self
            .conn
            .read_i32_le()
            .await?
            .try_into()
            .map_err(std::io::Error::other)?;

        let mut payload = Vec::with_capacity(
            length as usize - std::mem::size_of_val(&request_id) - std::mem::size_of_val(&r#type),
        );

        self.conn.read_until(0u8, &mut payload).await?;

        let packet = Packet {
            request_id,
            r#type,
            payload,
        };

        match request_id {
            -1 => Err(std::io::Error::other("authentication failed")),
            request_id => {
                self.responses.insert(request_id, packet);
                Ok(())
            }
        }
    }

    pub async fn login(&mut self, password: String) -> std::io::Result<Packet> {
        let packet = Packet::login(self.current_request_id, password);
        self.current_request_id += 1;
        self.send(packet).await
    }

    pub async fn command(&mut self, command: String) -> std::io::Result<Packet> {
        let packet = Packet::new(self.current_request_id, password);
        self.current_request_id += 1;
        self.send(packet).await
    }
}

fn main() {
    println!("Hello, world!");
}
