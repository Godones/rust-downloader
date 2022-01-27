#![allow(unused)]
use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};

type MessageId = u8;
type MessagePayload = Vec<u8>;

pub const MESSAGE_CHOKE: MessageId = 0; // 阻塞消息接收者
pub const MESSAGE_UNCHOKE: MessageId = 1; // 解除阻塞消息接收者
pub const MESSAGE_INTERESTED: MessageId = 2; // 表示有兴趣接收数据
pub const MESSAGE_NOTINTERSETED: MessageId = 3; // 表示没有有兴趣接收数据
pub const MESSAGE_HAVE: MessageId = 4; // 提醒消息接收者，发送者已经下载了一个块
pub const MESSAGE_BITFIELD: MessageId = 5; // 对发送者已经下载的片段进行编码
pub const MESSAGE_REQUEST: MessageId = 6; // 向消息接收者请求一个块
pub const MESSAGE_PIECE: MessageId = 7; // 传送满足请求的数据块
pub const MESSAGE_CANCEL: MessageId = 8; // 取消一个请求

#[derive(Default, Debug, Clone, PartialOrd, PartialEq)]
pub struct Message {
    // Message type
    pub id: MessageId,
    // Message payload
    pub payload: MessagePayload,
}

impl Message {
    pub fn new(id: MessageId) -> Self {
        Message {
            id,
            payload: vec![],
        }
    }

    pub fn new_with_payload(id: MessageId, payload: MessagePayload) -> Self {
        Message { id, payload }
    }

    /// 序列化Bitfield消息
    pub fn serialize(&self) -> Result<Vec<u8>> {
        // 消息长度，1表示的是一个byte，用来表示消息类型
        let message_len = 1 + self.payload.len();

        let mut serialized: Vec<u8> = vec![];

        // 添加消息长度
        serialized.write_u32::<BigEndian>(message_len as u32)?;

        // 添加消息类型
        serialized.push(self.id);

        // 添加 payload
        let mut payload = self.payload.clone();
        serialized.append(&mut payload);

        Ok(serialized)
    }
}

/// 反序列化得到的内容
pub fn deserialize_message(message_buf: &Vec<u8>, message_len: usize) -> Result<Message> {
    // 消息类型
    let id: MessageId = message_buf[0];
    // 消息 payload
    let payload: MessagePayload = message_buf[1..message_len].to_vec();
    // 构建 message
    let message: Message = Message::new_with_payload(id, payload);

    Ok(message)
}

#[cfg(test)]
mod message_test {
    use crate::bittorrent::message::{deserialize_message, Message};

    #[test]
    fn test_serialize_and_deserialize_correct() {
        let message = Message::new_with_payload(7, vec![1, 2, 4, 4, 5]);
        let serialized = message.serialize().unwrap();
        let deserialized = deserialize_message(&vec![7, 1, 2, 4, 4, 5], 6).unwrap();
        assert_eq!(message, deserialized);
    }
}
