use anyhow::Result;
const PROTOCOL_ID: &str = "BitTorrent protocol";

/// 握手需要发送的内容
pub struct Handshake {
    // 协议标识符的长度，始终为19 byte
    pub pstrlen: usize,
    // 协议标识符，称为pstr，始终为BitTorrent protocol
    pub pstr: Vec<u8>,
    // 八个保留字节，都设置为0
    pub reserved: Vec<u8>,
    // 计算出的信息哈希值，用于确定所需的文件
    pub info_hash: Vec<u8>,
    // 识别自己的Peer ID
    pub peer_id: Vec<u8>,
}

impl Handshake {
    pub fn new(peer_id: Vec<u8>, info_hash: Vec<u8>) -> Self {
        let pstr = String::from(PROTOCOL_ID).into_bytes();
        let pstrlen = pstr.len();
        let reserved: Vec<u8> = vec![0; 8];
        Handshake {
            pstrlen,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

    /// 反序列化发出的内容
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut serialized: Vec<u8> = vec![];

        serialized.push(self.pstrlen as u8);

        let mut pstr: Vec<u8> = self.pstr.clone();
        serialized.append(&mut pstr);

        let mut reserved: Vec<u8> = self.reserved.clone();
        serialized.append(&mut reserved);

        let mut info_hash: Vec<u8> = self.info_hash.clone();
        serialized.append(&mut info_hash);

        let mut peer_id: Vec<u8> = self.peer_id.clone();
        serialized.append(&mut peer_id);
        Ok(serialized)
    }
}
// 反序列化收到的内容
// 收到的内容应该与我们发出的格式相同
pub fn deserialize_handshake(buf: &Vec<u8>, pstrlen: usize) -> Result<Handshake> {
    let pstr = buf[0..pstrlen].to_vec();
    let reserved = buf[pstrlen..(pstrlen + 8)].to_vec();
    let info_hash = buf[(pstrlen + 8)..(pstrlen + 8 + 20)].to_vec();
    let peer_id = buf[(pstrlen + 8 + 20)..].to_vec();
    let handshake = Handshake {
        pstrlen,
        pstr,
        reserved,
        info_hash,
        peer_id,
    };
    Ok(handshake)
}
