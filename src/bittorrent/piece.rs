/// PieceWork structure.
#[derive(Default, Debug, Clone)]
pub struct PieceWork {
    // Piece 索引
    pub index: u32,
    // Piece 哈希值
    pub hash: Vec<u8>,
    // Piece 长度
    pub length: u32,
    // Piece 数据
    pub data: Vec<u8>,
    // Requests 索引
    pub requests: u32,
    // 请求到的数据大小
    pub requested: u32,
    // 下载了的数据大小
    pub downloaded: u32,
}

/// PieceResult structure.
#[derive(Default, Debug, Clone)]
pub struct PieceResult {
    pub index: u32,
    // Piece 长度
    pub length: u32,
    // Piece 数据
    pub data: Vec<u8>,
}

impl PieceWork {
    pub fn new(index: u32, hash: Vec<u8>, length: u32) -> PieceWork {
        PieceWork {
            index,
            hash,
            length,
            data: vec![0; length as usize],
            requests: 0,
            requested: 0,
            downloaded: 0,
        }
    }
}

impl PieceResult {
    pub fn new(index: u32, length: u32, data: Vec<u8>) -> PieceResult {
        PieceResult {
            index,
            length,
            data,
        }
    }
}
