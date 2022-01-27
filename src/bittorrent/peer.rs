extern crate anyhow;

use super::torrent::*;

use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ReadBytesExt};

use std::io::Cursor;
use std::net::Ipv4Addr;

const PEER_SIZE: usize = 6;

type PeerId = u32;

/// Peer定义
#[derive(Clone)]
pub struct Peer {
    pub id: PeerId,
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl Peer {
    pub fn new() -> Peer {
        Peer {
            id: 0,
            ip: Ipv4Addr::new(1, 1, 1, 1),
            port: 0,
        }
    }
}

impl Torrent {
    /// 构建所有的Peer的信息
    /// 总共6bytes
    /// 前4byte为ip地址，后两个字节为端口
    pub fn build_peers(&self, tracker_peers: Vec<u8>) -> Result<Vec<Peer>> {
        // 检查返回数据的合法性
        if tracker_peers.len() % PEER_SIZE != 0 {
            return Err(anyhow!("received invalid peers from tracker"));
        }

        // 获取peers数量
        let nb_peers = tracker_peers.len() / PEER_SIZE;

        // 建立peer
        let mut peers: Vec<Peer> = vec![Peer::new(); nb_peers];
        let mut port = vec![];

        for i in 0..nb_peers {
            // 建立 peer ID
            peers[i].id = i as u32;
            let offset = i * PEER_SIZE;
            // Add peer IP address
            peers[i].ip = Ipv4Addr::new(
                tracker_peers[offset],
                tracker_peers[offset + 1],
                tracker_peers[offset + 2],
                tracker_peers[offset + 3],
            );
            port.push(tracker_peers[offset + 4]);
            port.push(tracker_peers[offset + 5]);
            let mut port_cursor = Cursor::new(port);
            // Add peer port
            peers[i].port = port_cursor.read_u16::<BigEndian>()?;
            port = vec![];
        }

        Ok(peers)
    }
}

mod peer_test {
    use crate::bittorrent::torrent::Torrent;

    #[test]
    fn test_build_peer_success() {
        let torrent = Torrent::new();
        let peerinfo = vec![192, 165, 1, 21, 0, 1];
        let answer = torrent.build_peers(peerinfo).unwrap();
        assert_eq!(answer.len(), 1);
    }
    #[test]
    #[should_panic]
    fn test_build_peer_fail() {
        let torrent = Torrent::new();
        let peerinfo = vec![192, 165, 1, 21, 12];
        let _ = torrent.build_peers(peerinfo).unwrap();
    }
}
