use crate::http::http::HttpDownloader;
use crate::http::parser::CommandArgument;

mod parser;
pub mod http;


pub async fn download(){
    let mut command = CommandArgument::new();
    if let Ok(()) = command.parse(){
        //如果有实际的url则开始下载
        let downloader = HttpDownloader::new();
        let urls = command.get_url();
        let mut downloader = downloader.set_concurrency(command.get_concurrency().unwrap())
            .set_output_path(command.get_output_path().unwrap());
        for url in urls {
            downloader = downloader.set_url(url);
            downloader.download().await.unwrap();
        }
    }
}