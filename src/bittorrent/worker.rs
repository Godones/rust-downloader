use crate::bittorrent::client::*;
use crate::bittorrent::message::*;
use crate::bittorrent::peer::*;
use crate::bittorrent::piece::*;

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};
use crypto::digest::Digest;
use crypto::sha1::Sha1;

// 最大请求数量
const NB_REQUESTS_MAX: u32 = 5;

//每次请求的大小
const BLOCK_SIZE_MAX: u32 = 16384;

/// 生产者定义
pub struct Worker {
    peer: Peer,
    peer_id: Vec<u8>,
    info_hash: Vec<u8>,
    work_chan: (Sender<PieceWork>, Receiver<PieceWork>),
    result_chan: (Sender<PieceResult>, Receiver<PieceResult>),
}

impl Worker {
    pub fn new(
        peer: Peer,
        peer_id: Vec<u8>,
        info_hash: Vec<u8>,
        work_chan: (Sender<PieceWork>, Receiver<PieceWork>),
        result_chan: (Sender<PieceResult>, Receiver<PieceResult>),
    ) -> Result<Worker> {
        let worker = Worker {
            peer,
            peer_id,
            info_hash,
            work_chan,
            result_chan,
        };

        Ok(worker)
    }

    /// 启动worker.
    pub fn start_download(&self) {
        let peer_copy = self.peer.clone();
        let peer_id_copy = self.peer_id.clone();
        let info_hash_copy = self.info_hash.clone();

        // 建立客户端
        let mut client = match Client::new(peer_copy, peer_id_copy, info_hash_copy) {
            Ok(client) => client,
            Err(_) => return,
        };

        // 设置连接超时时间
        if client.set_connection_timeout(10).is_err() {
            return;
        }

        // 第一握手确认
        if client.handshake_with_peer().is_err() {
            return;
        }

        // 握手完成后，双方开始传输内容
        // 此时消息的格式发生了改变
        //消息以长度指示符开头，该指示符告诉我们该消息将有多少字节长，它是一个32位整数，
        // 意味着它是由 4 个 byte 按序排列的字节组成。
        // 下一个字节，即ID，告诉我们正在接收的消息类型（Message Type）。
        // 最后，可选的有效 payload 将填充消息的剩余长度.
        // 第二次读取peer发送的消息
        if client.read_bitfield().is_err() {
            return;
        }

        // 本机发送unchoke消息代表自己准备好了，可以进行消息传输了
        // 握手完成后的刚开始，我们被其他 Peer
        // 认为状态是阻塞的（chocked）,我们需要发送一条解锁消息
        if client.send_unchoke().is_err() {
            return;
        }

        // 发送 Interested 消息，代表自己要开始下载文件了
        if client.send_interested().is_err() {
            return;
        }

        loop {
            // 从work channel读取piece
            let mut piece_work: PieceWork = match self.work_chan.1.recv() {
                Ok(piece_work) => piece_work,
                Err(_) => {
                    error!("Error: could not receive piece from channel");
                    return;
                }
            };

            if !client.has_piece(piece_work.index) {
                // 如果本地没有这个piece就再发送到channel中
                if self.work_chan.0.send(piece_work).is_err() {
                    error!("Error: could not send piece to channel");
                    return;
                }
                continue;
            }

            // 下载piece
            if self.download_piece(&mut client, &mut piece_work).is_err() {
                // Resend piece to work channel
                if self.work_chan.0.send(piece_work).is_err() {
                    error!("Error: could not send piece to channel");
                    return;
                }
                return;
            }

            // 校验下载的piece
            if self.verify_piece_integrity(&mut piece_work).is_err() {
                // 校验错误重新请求
                if self.work_chan.0.send(piece_work).is_err() {
                    error!("Error: could not send piece to channel");
                    return;
                }
                continue;
            }

            // 通知已经下载了这个piece
            if client.send_have(piece_work.index).is_err() {
                error!("Error: could not notify peer that piece was downloaded");
            }

            // 将下载的piece发送到result channel中
            let piece_result =
                PieceResult::new(piece_work.index, piece_work.length, piece_work.data);
            if self.result_chan.0.send(piece_result).is_err() {
                error!("Error: could not send piece to channel");
                return;
            }
        }
    }

    fn download_piece(&self, client: &mut Client, piece_work: &mut PieceWork) -> Result<()> {
        // 设置连接超时时间
        // 对于下载资源来说要较长
        client.set_connection_timeout(120)?;

        // 重置
        piece_work.requests = 0;
        piece_work.requested = 0;
        piece_work.downloaded = 0;

        // Download torrent piece
        while piece_work.downloaded < piece_work.length {
            // 如果客户端被阻塞
            if !client.is_choked() {
                while piece_work.requests < NB_REQUESTS_MAX
                    && piece_work.requested < piece_work.length
                {
                    // 读取内容
                    let mut block_size = BLOCK_SIZE_MAX;
                    let remaining = piece_work.length - piece_work.requested;
                    if remaining < BLOCK_SIZE_MAX {
                        block_size = remaining;
                    }

                    // 请求内容
                    client.send_request(piece_work.index, piece_work.requested, block_size)?;

                    // 更新发送的请求数目
                    piece_work.requests += 1;

                    // 更新请求的数据
                    piece_work.requested += block_size;
                }
            }

            // 监听是否又消息来
            let message: Message = client.read_message()?;

            // 解析消息
            match message.id {
                MESSAGE_CHOKE => client.read_choke(),       //阻塞客户端
                MESSAGE_UNCHOKE => client.read_unchoke(),   //解除阻塞
                MESSAGE_HAVE => client.read_have(message)?, //本地已经下载
                MESSAGE_PIECE => client.read_piece(message, piece_work)?, //下载一个资源快
                _ => info!("received unknown message from peer"),
            }
        }
        info!("Successfully downloaded piece {:?}", piece_work.index);
        Ok(())
    }

    fn verify_piece_integrity(&self, piece_work: &mut PieceWork) -> Result<()> {
        let mut hasher = Sha1::new();
        hasher.input(&piece_work.data);
        let hex = hasher.result_str();
        // 编码字符串
        let decoded: Vec<u8> = hex::decode(hex)?;
        // 比较是否相同
        if decoded != piece_work.hash {
            return Err(anyhow!(
                "could not verify integrity of piece downloaded from peer"
            ));
        }
        info!(
            "Successfully verified integrity of piece {:?}",
            piece_work.index
        );

        Ok(())
    }
}
