pub mod client;
pub mod handshake;
pub mod message;
pub mod parser;
pub mod peer;
pub mod piece;
pub mod torrent;
pub mod worker;

use crate::bittorrent::parser::CommandArgument;
use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use torrent::*;

async fn run(torrent: &str, file: &str) -> Result<()> {
    // 检查文件是否存在
    if !Path::new(&torrent).exists() {
        return Err(anyhow!("could not find torrent"));
    } else {
        let torrent_filepath = PathBuf::from(torrent);
        let output_filepath = PathBuf::from(file);

        // 新建下载文件
        let mut output_file = match File::create(output_filepath) {
            Ok(file) => file,
            Err(_) => return Err(anyhow!("could not create file")),
        };

        // 打开torrent文件并开始下载
        let mut torrent = Torrent::new();
        if torrent.open(torrent_filepath).await.is_err() {
            return Err(anyhow!("could not open file"));
        };
        let data: Vec<u8> = torrent.download()?;

        // 保存文件
        if output_file.write(&data).is_err() {
            return Err(anyhow!("could not write data to file"));
        }

        println!("Saved in {:?}.", file);
    }

    Ok(())
}

pub async fn execute() {
    //初始化日志
    pretty_env_logger::init_timed();
    // 解析参数
    let mut command = CommandArgument::new();
    command.parse();
    let file = command.get_torrent();
    let target_path = command.get_target_path();
    if let Err(error) = run(file, target_path).await {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
    std::process::exit(0);
}
