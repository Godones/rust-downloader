extern crate anyhow;
extern crate crypto;
extern crate hex;
extern crate serde;
extern crate serde_bencode;
extern crate url;

use crate::bittorrent::peer::*;
use crate::bittorrent::piece::*;
use crate::bittorrent::worker::*;

use anyhow::{anyhow, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_bencode::{de, ser};
use serde_bytes::ByteBuf;
use std::str;
use url::Url;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

const PORT: u16 = 6881; //监听端口
const SHA1_HASH_SIZE: usize = 20; //SHA1长度

/// Torrent structure.
#[derive(Default, Clone)]
pub struct Torrent {
    // 主服务器的url
    announce: String,
    //整个文件的hash校验值
    info_hash: Vec<u8>,
    // SHA-1 hashes of each pieces
    pieces_hashes: Vec<Vec<u8>>,
    // 每个piece的大小
    piece_length: u32,
    // 文件总大小
    length: u32,
    // Suggested filename where to save the file
    name: String,
    // 标识本机的id
    peer_id: Vec<u8>,
    // Peers
    peers: Vec<Peer>,
}

/// BencodeInfo structure.
#[derive(Deserialize, Serialize)]
struct BencodeInfo {
    // 所有piece的hash
    #[serde(rename = "pieces")]
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: u32,
    #[serde(rename = "length")]
    length: u32,
    /// 文件名称
    #[serde(rename = "name")]
    name: String,
}

/// BencodeTorrent structure.
#[derive(Deserialize, Serialize)]
struct BencodeTorrent {
    #[serde(default)]
    // URL of the tracker
    announce: String,
    info: BencodeInfo,
}

/// tracker服务器发回的消息
/// 包含interval和peers
#[derive(Debug, Deserialize, Serialize)]
struct BencodeTracker {
    //告诉客户端间隔多久再向服务端发一次请求
    interval: u32,
    // peers 字段包含同伴的 peer_id、ip、port 等信息
    peers: ByteBuf,
}

impl BencodeInfo {
    /// HASH编码信息得到文件的唯一标识
    fn hash(&self) -> Result<Vec<u8>> {
        // 序列化编码信息
        let buf: Vec<u8> = ser::to_bytes::<BencodeInfo>(self)?;
        // SHA1加密
        let mut hasher = Sha1::new();
        hasher.input(&buf);
        // 得到加密字符串
        let hex = hasher.result_str();
        // 将字符串解码为二进制
        let decoded: Vec<u8> = hex::decode(hex)?;
        Ok(decoded)
    }

    /// 将所有piece的hash校验值切分
    fn split_pieces_hashes(&self) -> Result<Vec<Vec<u8>>> {
        let pieces = self.pieces.to_owned();
        let nb_pieces = pieces.len();

        // 检查是否是合法的序列
        if nb_pieces % SHA1_HASH_SIZE != 0 {
            return Err(anyhow!("torrent is invalid"));
        }
        let nb_hashes = nb_pieces / SHA1_HASH_SIZE;
        let mut hashes: Vec<Vec<u8>> = vec![vec![0; 20]; nb_hashes];

        // 切分序列
        for i in 0..nb_hashes {
            hashes[i] = pieces[i * SHA1_HASH_SIZE..(i + 1) * SHA1_HASH_SIZE].to_vec();
        }

        Ok(hashes)
    }
}

impl Torrent {
    pub fn new() -> Self {
        Default::default()
    }

    /// 打开torrent文件构建
    pub async  fn open(&mut self, filepath: PathBuf) -> Result<()> {
        let mut file = match File::open(filepath) {
            Ok(file) => file,
            Err(_) => return Err(anyhow!("could not open torrent")),
        };
        // Read torrent content in a buffer
        let mut buf = vec![];
        if file.read_to_end(&mut buf).is_err() {
            return Err(anyhow!("could not read torrent"));
        }
        // 从缓冲区内容序列化
        let bencode = match de::from_bytes::<BencodeTorrent>(&buf) {
            Ok(bencode) => bencode,
            Err(_) => return Err(anyhow!("could not decode torrent")),
        };

        // 生成一个随机peer
        let mut peer_id: Vec<u8> = vec![0; 20];
        let mut rng = rand::thread_rng();
        for x in peer_id.iter_mut() {
            *x = rng.gen();
        }

        // 设置所有的信息
        self.announce = bencode.announce.to_owned();
        self.info_hash = bencode.info.hash()?;
        self.pieces_hashes = bencode.info.split_pieces_hashes()?;
        self.piece_length = bencode.info.piece_length;
        self.length = bencode.info.length;
        self.name = bencode.info.name.to_owned();
        self.peer_id = peer_id.clone();
        self.peers = self.request_peers(peer_id, PORT).await.unwrap();
        Ok(())
    }

    /// 向track服务器发送请求获取所有peer的信息
    async fn request_peers(&self, peer_id: Vec<u8>, port: u16) -> Result<Vec<Peer>> {
        // 建立请求url
        let tracker_url = match self.build_tracker_url(peer_id, port) {
            Ok(url) => url,
            Err(_) => return Err(anyhow!("could not build tracker url")),
        };

        // 建立http客户端
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(15)).build(){
            Ok(client) => client,
            Err(_) => return Err(anyhow!("could not connect to tracker")),
        };

        // 发送请求
        let response = match client.get(&tracker_url).send().await {
            Ok(response) => match response.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("could not read response from tracker")),
            },
            Err(_) => return Err(anyhow!("could not send request to tracker")),
        };

        // 反序列化返回的信息
        let tracker_bencode = match de::from_bytes::<BencodeTracker>(&response) {
            Ok(bencode) => bencode,
            Err(_) => return Err(anyhow!("could not decode tracker response")),
        };

        // 建立peers信息
        let peers: Vec<Peer> = match self.build_peers(tracker_bencode.peers.to_vec()) {
            Ok(peers) => peers,
            Err(_) => return Err(anyhow!("could not build peers")),
        };
        Ok(peers)
    }

    /// 构建 tracker URL.
    fn build_tracker_url(&self, peer_id: Vec<u8>, port: u16) -> Result<String> {
        // 解析文件中的tracker url
        let mut base_url = match Url::parse(&self.announce) {
            Ok(url) => url,
            Err(_) => return Err(anyhow!("could not parse tracker url")),
        };

        // 添加参数
        base_url
            // Add info hash
            .query_pairs_mut()
            .encoding_override(Some(&|input| {
                if input != "!" {
                    Cow::Borrowed(input.as_bytes())
                } else {
                    Cow::Owned(self.info_hash.clone())
                }
            }))
            .append_pair("info_hash", "!");
        base_url
            // Add peer id
            .query_pairs_mut()
            .encoding_override(Some(&|input| {
                if input != "!" {
                    Cow::Borrowed(input.as_bytes())
                } else {
                    Cow::Owned(peer_id.clone())
                }
            }))
            .append_pair("peer_id", "!");
        base_url
            .query_pairs_mut()
            // 添加监听的端口
            .append_pair("port", &port.to_string())
            // 添加上传的大小
            .append_pair("uploaded", "0")
            // 添加下载的大小
            .append_pair("downloaded", "0")
            // 添加compact
            .append_pair("compact", "1")
            // 添加仍然需要下载的数量
            .append_pair("left", &self.length.to_string());
        Ok(base_url.to_string())
    }

    /// 下载文件
    pub fn download(&self) -> Result<Vec<u8>> {
        println!(
            "Downloading {:?} ({:?} pieces)",
            self.name,
            self.pieces_hashes.len(),
        );

        // 创建无限容量的channel-->工作channel
        let work_chan: (Sender<PieceWork>, Receiver<PieceWork>) = unbounded();

        // 下载结果channel
        let result_chan: (Sender<PieceResult>, Receiver<PieceResult>) = unbounded();

        // 新建piece生产者并发送到channel中
        // 每个piece对应一个生产者
        for index in 0..self.pieces_hashes.len() {
            // 创建piece
            let piece_index = index as u32;
            let piece_hash = self.pieces_hashes[index].clone();
            let piece_length = self.get_piece_length(piece_index)?;
            let piece_work = PieceWork::new(piece_index, piece_hash, piece_length);
            // 将生产者送入channel中
            if work_chan.0.send(piece_work).is_err() {
                return Err(anyhow!("Error: could not send piece to channel"));
            }
        }

        // 初始化生产者
        let peers = self.peers.to_owned();
        for peer in peers {
            let peer_copy = peer.clone();
            let peer_id_copy = self.peer_id.clone();//本机id
            let info_hash_copy = self.info_hash.clone();
            let work_chan_copy = work_chan.clone();
            let result_chan_copy = result_chan.clone();

            // 创建工作者
            let worker = Worker::new(
                peer_copy,
                peer_id_copy,
                info_hash_copy,
                work_chan_copy,
                result_chan_copy,
            )?;

            // 在新的线程工作
            thread::spawn(move || {
                worker.start_download();
            });
        }

        // 创建进度条
        let pb = ProgressBar::new(self.length as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} {bytes}/{total_bytes} [{bar:40.cyan/blue}] {percent}%")
                .progress_chars("#>-"),
        );

        // 建立下载文件
        let mut data: Vec<u8> = vec![0; self.length as usize];
        let mut nb_pieces_downloaded = 0;
        while nb_pieces_downloaded < self.pieces_hashes.len() {
            let piece_result: PieceResult = match result_chan.1.recv() {
                Ok(piece_result) => piece_result,
                Err(_) => return Err(anyhow!("Error: could not receive piece from channel")),
            };

            // Copy piece data
            let begin: u32 = piece_result.index * self.piece_length;
            for i in 0..piece_result.length as usize {
                data[begin as usize + i] = piece_result.data[i];
            }
            // 更新进度条
            pb.inc(piece_result.length as u64);
            // 更新下载的内容
            nb_pieces_downloaded += 1;
        }
        Ok(data)
    }

    /// 获取piece长度
    /// 主要是为了防止最后一个piece长度与文件中的不一样
    fn get_piece_length(&self, index: u32) -> Result<u32> {
        let begin: u32 = index * self.piece_length;
        let mut end: u32 = begin + self.piece_length;
        if end > self.length {
            end = self.length;
        }

        Ok(end - begin)
    }
}
