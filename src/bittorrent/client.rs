use crate::bittorrent::handshake::*;
use crate::bittorrent::message::*;
use crate::bittorrent::piece::*;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::bittorrent::peer::Peer;
use std::io::{Cursor, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

/// 客户端定义
pub struct Client {
    peer: Peer,
    // Torrent peer id
    peer_id: Vec<u8>,
    // Torrent info hash
    info_hash: Vec<u8>,
    // Connection to peer
    conn: TcpStream,
    // Bitfield 可以理解成一个二进制 bit 数组，数组值为1 ，就代表有这个块，0代表没有
    bitfield: Vec<u8>,
    // Peer 是否断开此链接
    choked: bool,
}

impl Client {
    pub fn new(peer: Peer, peer_id: Vec<u8>, info_hash: Vec<u8>) -> Result<Client> {
        // 与peer建立链接
        let peer_socket = SocketAddr::new(IpAddr::V4(peer.ip), peer.port);
        let conn = match TcpStream::connect_timeout(&peer_socket, Duration::from_secs(15)) {
            Ok(conn) => conn,
            Err(_) => return Err(anyhow!("could not connect to peer")),
        };
        info!("Connected to peer {:?}", peer.id);
        let client = Client {
            peer,
            peer_id,
            info_hash,
            conn,
            bitfield: vec![],
            choked: true,
        };

        Ok(client)
    }

    // 返回客户端是否被阻塞
    pub fn is_choked(&self) -> bool {
        self.choked
    }

    /// &公有函数\\
    pub fn has_piece(&self, index: u32) -> bool {
        let byte_index = index / 8;
        let offset = index % 8;

        // bitfield是u8类型的，每个bit都代表这个位置上的内容存在与否
        // index = 100
        // byte_index= 100/8 --->位于第几个u8
        // offset--->位于u8的第几个位置
        if byte_index < self.bitfield.len() as u32 {
            // Check for piece index into bitfield
            return self.bitfield[byte_index as usize] >> (7 - offset) as u8 & 1 != 0;
        }
        false
    }

    /// 设置piece
    pub fn set_piece(&mut self, index: u32) {
        let byte_index = index / 8;
        let offset = index % 8;

        // 新建一个bitfield放置piece内容
        let mut bitfield: Vec<u8> = self.bitfield.to_vec();

        if byte_index < self.bitfield.len() as u32 {
            bitfield[byte_index as usize] |= (1 << (7 - offset)) as u8;
            self.bitfield = bitfield;
        }
    }

    /// 设置超时时间
    pub fn set_connection_timeout(&self, secs: u64) -> Result<()> {
        // Set write timeout
        if self
            .conn
            .set_write_timeout(Some(Duration::from_secs(secs)))
            .is_err()
        {
            return Err(anyhow!("could not set write timeout"));
        }

        // Set read timeout
        if self
            .conn
            .set_read_timeout(Some(Duration::from_secs(secs)))
            .is_err()
        {
            return Err(anyhow!("could not set read timeout"));
        }

        Ok(())
    }

    /// 完成握手
    pub fn handshake_with_peer(&mut self) -> Result<()> {
        // 创建握手内容
        let peer_id = self.peer_id.clone();
        let info_hash = self.info_hash.clone();
        let handshake = Handshake::new(peer_id, info_hash);

        // 发送序列化信息
        let handshake_encoded: Vec<u8> = handshake.serialize()?;
        if self.conn.write(&handshake_encoded).is_err() {
            return Err(anyhow!("could not send handshake to peer"));
        }

        // 读取收到的序列化信息
        let handshake_len: usize = self.read_handshake_len()?;
        //读取序列化信息长度
        let mut handshake_buf: Vec<u8> = vec![0; 48 + handshake_len];
        if self.conn.read_exact(&mut handshake_buf).is_err() {
            return Err(anyhow!("could not read handshake received from peer"));
        }

        //校验两端的文件hash值是否一样
        let handshake_decoded: Handshake = deserialize_handshake(&handshake_buf, handshake_len)?;
        if handshake_decoded.info_hash != self.info_hash {
            return Err(anyhow!("invalid handshake received from peer"));
        }
        Ok(())
    }

    /// 读取握手返回的内容长度
    fn read_handshake_len(&mut self) -> Result<usize> {
        let mut buf = [0; 1];
        if self.conn.read_exact(&mut buf).is_err() {
            return Err(anyhow!(
                "could not read handshake length received from peer"
            ));
        }
        // 检查长度是否正确
        let handshake_len = buf[0];
        if handshake_len == 0 {
            return Err(anyhow!("invalid handshake length received from peer"));
        }

        Ok(handshake_len as usize)
    }

    /// 读取bitfield内容
    pub fn read_message(&mut self) -> Result<Message> {
        let message_len: usize = self.read_message_len()?;
        //如果未收到消息，即长度为0，保持连接
        if message_len == 0 {
            info!("Receive KEEP_ALIVE from peer {:?}", self.peer.id);
            return Err(anyhow!("keep-alive"));
        }
        let mut message_buf: Vec<u8> = vec![0; message_len];
        if self.conn.read_exact(&mut message_buf).is_err() {
            return Err(anyhow!("could not read message received from peer"));
        }
        // 反序列化
        let message: Message = deserialize_message(&message_buf, message_len)?;
        Ok(message)
    }

    /// 获取消息长度
    fn read_message_len(&mut self) -> Result<usize> {
        let mut buf = vec![0; 4];
        if self.conn.read_exact(&mut buf).is_err() {
            return Err(anyhow!("could not read message length received from peer"));
        }

        let mut cursor = Cursor::new(buf);
        let message_len = cursor.read_u32::<BigEndian>()?;

        Ok(message_len as usize)
    }

    /// Read CHOKE message from remote peer.
    pub fn read_choke(&mut self) {
        info!("Receive MESSAGE_CHOKE from peer {:?}", self.peer.id);
        self.choked = true
    }

    /// 发送 解除阻塞的消息
    pub fn send_unchoke(&mut self) -> Result<()> {
        let message: Message = Message::new(MESSAGE_UNCHOKE);
        let message_encoded = message.serialize()?;
        info!("Send MESSAGE_UNCHOKE to peer {:?}", self.peer.id);

        if self.conn.write(&message_encoded).is_err() {
            return Err(anyhow!("could not send MESSAGE_UNCHOKE to peer"));
        }

        Ok(())
    }

    /// 设置本机未阻塞
    pub fn read_unchoke(&mut self) {
        info!("Receive MESSAGE_UNCHOKE from peer {:?}", self.peer.id);
        self.choked = false
    }

    /// 发送有兴趣下载消息
    pub fn send_interested(&mut self) -> Result<()> {
        let message: Message = Message::new(MESSAGE_INTERESTED);
        let message_encoded = message.serialize()?;

        info!("Send MESSAGE_INTERESTED to peer {:?}", self.peer.id);

        if self.conn.write(&message_encoded).is_err() {
            return Err(anyhow!("could not send MESSAGE_INTERESTED to peer"));
        }
        Ok(())
    }

    /// 发送已经下载某个piece的消息
    pub fn send_have(&mut self, index: u32) -> Result<()> {
        let mut payload: Vec<u8> = vec![];
        payload.write_u32::<BigEndian>(index)?;

        let message: Message = Message::new_with_payload(MESSAGE_HAVE, payload);
        let message_encoded = message.serialize()?;

        info!("Send MESSAGE_HAVE to peer {:?}", self.peer.id);
        if self.conn.write(&message_encoded).is_err() {
            return Err(anyhow!("could not send MESSAGE_HAVE to peer"));
        }

        Ok(())
    }

    pub fn read_have(&mut self, message: Message) -> Result<()> {
        info!("Receive MESSAGE_HAVE from peer {:?}", self.peer.id);
        // 检查消息合法性
        if message.id != MESSAGE_HAVE || message.payload.to_vec().len() != 4 {
            return Err(anyhow!("received invalid MESSAGE_HAVE from peer"));
        }
        //读取消息索引
        let mut payload_cursor = Cursor::new(message.payload.to_vec());
        let index = payload_cursor.read_u32::<BigEndian>()?;
        // 更新索引
        self.set_piece(index);
        Ok(())
    }

    /// 读取bitfield消息
    pub fn read_bitfield(&mut self) -> Result<()> {
        info!("Receive MESSAGE_BITFIELD from peer {:?}", self.peer.id);
        let message: Message = self.read_message()?;
        //只接收对下载piece编码的消息
        if message.id != MESSAGE_BITFIELD {
            return Err(anyhow!("received invalid MESSAGE_BITFIELD from peer"));
        }

        // 更新内容
        self.bitfield = message.payload.to_vec();
        Ok(())
    }

    pub fn send_request(&mut self, index: u32, begin: u32, length: u32) -> Result<()> {
        let mut payload: Vec<u8> = vec![];
        payload.write_u32::<BigEndian>(index)?;
        payload.write_u32::<BigEndian>(begin)?;
        payload.write_u32::<BigEndian>(length)?;

        let message: Message = Message::new_with_payload(MESSAGE_REQUEST, payload);
        let message_encoded = message.serialize()?;

        info!(
            "Send MESSAGE_REQUEST for piece {:?} [{:?}:{:?}] to peer {:?}",
            index,
            begin,
            begin + length,
            self.peer.id
        );

        if self.conn.write(&message_encoded).is_err() {
            return Err(anyhow!("could not send MESSAGE_REQUEST to peer"));
        }
        Ok(())
    }

    pub fn read_piece(&mut self, message: Message, piece_work: &mut PieceWork) -> Result<()> {
        info!("Receive MESSAGE_PIECE from peer {:?}", self.peer.id);

        // 检查消息合法性
        if message.id != MESSAGE_PIECE || message.payload.to_vec().len() < 8 {
            return Err(anyhow!("received invalid MESSAGE_HAVE from peer"));
        }

        let payload: Vec<u8> = message.payload.to_vec();

        let mut payload_cursor = Cursor::new(&payload[0..4]);
        let index = payload_cursor.read_u32::<BigEndian>()?;

        if index != piece_work.index {
            return Err(anyhow!("received invalid piece from peer"));
        }

        // 获得这个piece的index
        let mut payload_cursor = Cursor::new(&payload[4..8]);
        let begin: u32 = payload_cursor.read_u32::<BigEndian>()?; //piece的开始位置

        let block: Vec<u8> = payload[8..].to_vec();
        let block_len: u32 = block.len() as u32; //长度

        // Check if byte offset is valid
        if begin + block_len > piece_work.length as u32 {
            return Err(anyhow!(
                "received invalid byte offset within piece from peer"
            ));
        }

        info!(
            "Download piece {:?} [{:?}:{:?}] from peer {:?}",
            index,
            begin,
            begin + block_len,
            self.peer.id
        );

        // 将收到的内容保存
        for i in 0..block_len {
            piece_work.data[begin as usize + i as usize] = block[i as usize];
        }

        // 更新已经下载的数量
        piece_work.downloaded += block_len;

        // 更新已经请求的数量
        piece_work.requests -= 1;

        Ok(())
    }
}
