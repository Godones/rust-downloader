#![allow(dead_code)]
use colorful::{Color, Colorful};
use futures_util::future::join_all;
use futures_util::{StreamExt};
use reqwest::header::RANGE;
use reqwest::{Response, StatusCode};
use std::cmp::min;
use std::fmt::{self, Formatter};
use std::io::SeekFrom;
use std::ops::Add;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
/// 文件下载器
pub struct HttpDownloader {
    url: Option<String>,         //下载链接
    concurrency: Option<u16>,    //线程数目
    output_path: Option<String>, //保存路径
    client: reqwest::Client,     //客户端
    count:usize,//记录下载的文件数量，用来生成没有文件名的文件
}

impl fmt::Display for HttpDownloader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "http_url:{:?}\n save_path:{:?}\nconcurrency:{:?}",
            self.url, self.output_path, self.concurrency
        )
    }
}
impl HttpDownloader {
    pub fn new() -> Self {
        Self {
            url: None,
            concurrency: Some(8),
            output_path: Some(String::from(".")),
            client: reqwest::Client::new(),
            count:0,
        }
    }
    /// 设置下载链接
    pub fn set_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }
    /// 设置保存路径
    pub fn set_output_path(mut self, output_path: String) -> Self {
        self.output_path = Some(output_path);
        self
    }
    ///设置线程数
    pub fn set_concurrency(mut self, concurrency: u16) -> Self {
        self.concurrency = Some(concurrency);
        self
    }
    /// 从相应的header中解析文件名称，如果不存在则设置一个默认名称download.bin
    fn parse_filename(&mut self, result: &Response) -> String {
        let head = result.headers();
        //默认名称
        let mut filepath = format!("download{}.bin",self.count);
        self.count +=1;
        //检查是否有对应的键值对
        if let Some(content) = head.get("content-disposition") {
            let str = content.to_str().unwrap();
            //按照;分割
            let str_split: Vec<&str> = str.split(';').collect();
            if str_split.len() > 1 {
                //检查是否含有filename=字段
                if str_split[1].to_lowercase().starts_with("filename=") {
                    let filename: Vec<&str> = str_split[1].split('=').collect();
                    if filename.len() > 1 {
                        // 提取名称
                        filepath = filename[1][1..filename[1].len() - 1].to_string()
                    }
                }
            }
        }
        // 将名称与路径结合
        let new_path = self.output_path.clone().unwrap();
        let new_path = new_path.add("/").add(filepath.as_ref());
        new_path
    }
    /// 异步发送请求
    async fn send_request_for_head(&self) -> Response {
        //只要请求head部分即可
        let result = self
            .client
            .head(self.url.as_ref().unwrap().clone())
            .send()
            .await
            .expect("Can't Request ok");
        //判断是否请求正确
        if result.status() != StatusCode::OK {
            panic!("Request Fail");
        }
        result
    }
    /// 发送请求获取 全部数据
    async fn send_request_for_alldata(&self) -> Response {
        // 对于不能多线程下载的文件发送请求不需要带上RANGE字段
        let result = self
            .client
            .get(self.url.as_ref().unwrap().clone())
            .send()
            .await
            .expect("Can't Request ok");
        if result.status() != StatusCode::OK {
            panic!("Request Fail");
        }
        result
    }
    /// 多线程需要请求部分数据
    async fn send_request_for_data(&self, start: u64, end: u64) -> Response {
        let result = self
            .client
            .get(self.url.as_ref().unwrap().clone())
            .header(RANGE, format!("bytes={}-{}", start, end))
            .send()
            .await
            .expect("Can't Request ok");
        if result.status() != StatusCode::PARTIAL_CONTENT {
            panic!("Request Fail");
        }
        result
    }
    /// 将资源大小按照线程数目分割
    fn split(&self, filesize: u64) -> Vec<(u64, u64)> {
        let concurrency = self.concurrency.unwrap() as u64;
        let mut partition: Vec<(u64, u64)> = Vec::new();
        let part = (filesize + concurrency - 1) / concurrency;
        for i in 0..concurrency as usize {
            partition.push((i as u64 * part, min(i as u64 * part + part, filesize)))
        }
        partition
    }
    /// 获取文件大小并确认是否支持多线程下载
    fn makesure_support_download(&self, head: &Response) -> Option<(bool, u64)> {
        // 获取文件大小
        let content_length = head.headers().get("content-length");
        let ranges_flag = match head.headers().get("accept-ranges") {
            None => false,
            Some(val) => val.to_str().unwrap().eq("bytes"),
        };
        // println!("{:?} {:?}",content_length,ranges_flag);
        let mut answer = (false, 0);
        if ranges_flag {
            //支持文件并发下载
            answer.0 = true;
        }
        if let Some(val) = content_length {
            let content_length = val.to_str().unwrap().parse::<u64>().unwrap();
            if content_length == 0 {
                println!("文件大小为0,请确认链接正确");
                return None;
            }
            answer.1 = content_length;
        }
        Some(answer)
    }
    /// 异步下载资源块
    async fn download_partition(
        &self,
        range: (u64, u64),
        file: Arc<Mutex<File>>,
        support: bool,
    ) -> Result<(), reqwest::Error> {
        // 获得当前时间
        let data_response;
        if support {
            data_response = self.send_request_for_data(range.0, range.1).await;
        } else {
            data_response = self.send_request_for_alldata().await;
        }
        // 流式请求资源

        let mut stream = data_response.bytes_stream();
        let mut data = Vec::new();
        let time_began = Instant::now();
        while let Some(item) = stream.next().await {
            let item = item.unwrap();
            data.push(item);
        }
        let time_cost = time_began.elapsed().as_secs_f64();
        let mut file = file.lock().await;
        // seek到文件指定位置
        file.seek(SeekFrom::Start(range.0))
            .await
            .expect("seek error");
        for data in data.iter_mut() {
            file.write_all_buf(&mut *data).await.expect("write error");
        }
        let filesize = (range.1 - range.0) as f64;
        // 答应下载速度
        let str = format!(
            "range:{}-{} downloaded speed:{:>4.2}MB/s",
            range.0,
            range.1,
            ((filesize as f64) / 1024.0 / 1024.0) as f64 / time_cost
        );
        println!("{}", str.gradient(Color::Green).bold());
        Ok(())
    }

    ///异步下载
    pub async fn download(&mut self) -> Result<(), reqwest::Error> {
        //异步发送请求
        let head = self.send_request_for_head().await;
        let content_range_length = self.makesure_support_download(&head);
        if content_range_length.is_none() {
            panic!("文件不可下载");
        }
        let content_range_length = content_range_length.unwrap(); //得到文件大小和是否支持并发下载
        let path = self.parse_filename(&head); //得到保存路径
        //新建一个资源文件
        let file = Arc::new(Mutex::new(
            File::create(path).await.expect("create file error"),
        ));
        println!("{}","download.......".color(Color::Red));
        if content_range_length.0 {
            //支持并发下载
            let partition = self.split(content_range_length.1);
            let mut futures = Vec::new();
            for iter in partition {
                let future = self.download_partition(iter, file.clone(), true);
                futures.push(future);
            }
            join_all(futures).await;
        } else {
            //不支持并发下载
            self.download_partition((0, content_range_length.1), file.clone(), false)
                .await
                .expect("error when download file");
        }
        println!("{}","download ok".color(Color::Red));
        Ok(())
    }
}

// /// 异步测试
#[cfg(test)]
#[allow(non_snake_case)]
mod HttpDownloadTest {
    use super::*;
    macro_rules! BLOCK {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }
    async fn test_httpdownload_success() {
        let url = "https://issuecdn.baidupcs.com/issue/netdisk/yunguanjia/BaiduNetdisk_7.2.8.9.exe";
        let mut download = HttpDownloader::new()
            .set_url(String::from(url))
            .set_concurrency(8)
            .set_output_path(".".to_string());
        download.download().await.unwrap();
    }
    #[test]
    fn test_download_success() {
        BLOCK!(test_httpdownload_success());
    }
}
