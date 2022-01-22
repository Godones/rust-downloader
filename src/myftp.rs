#![allow(dead_code)]
use async_ftp::FtpStream;
use async_std::fs::File;
use futures_util::AsyncWriteExt;
use std::str::from_utf8;

pub struct FTP {
    ftpstream: FtpStream,
}

impl FTP {
    /// 登录ftp服务器
    pub async fn login(address: &str, user: &str, password: &str) -> Self {
        let mut ftp_stream = FtpStream::connect(address).await.unwrap();
        ftp_stream.login(&user, &password).await.unwrap();
        println!("Login Ok!");
        FTP {
            ftpstream: ftp_stream,
        }
    }
    /// 打印当前目录文件
    pub async fn list(&mut self, path: Option<&str>) {
        let name_list = self.ftpstream.list(path).await.unwrap();
        name_list.into_iter().for_each(|name| {
            println!("{}", name);
        })
    }

    /// 进入某个目录下
    pub async fn cwd(&mut self, path: &str) {
        self.ftpstream.cwd(path).await.unwrap();
    }

    /// 下载某个文件到指定目录下
    pub async fn download(&mut self, filename: &str, target: &str) -> bool {
        let remote_file = self.ftpstream.simple_retr(filename).await.unwrap();
        let vec_data = remote_file.into_inner();
        let target_path = target.to_string()+filename;
        let mut file = File::create(target_path).await.unwrap();
        let file_content = from_utf8(vec_data.as_slice()).unwrap();
        file.write_all(file_content.as_bytes()).await.unwrap();
        true
    }

    /// 断开链接
    pub async fn disconnect(&mut self){
        self.ftpstream.quit().await.unwrap();
    }
}



mod ftptest{
    use std::fs;
    use crate::myftp::FTP;
    #[allow(unused_macros)]
    macro_rules! BLOCK {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }
    async fn async_ftp_login(){
        let address = "192.168.1.4:21";
        let user = "God";
        let passward = "52531225253.";
        let mut ftp = FTP::login(address, user, passward).await;
        ftp.disconnect().await;
    }

    async fn async_ftp_list() {
        let address = "192.168.1.4:21";
        let user = "God";
        let passward = "52531225253.";
        let mut  ftp = FTP::login(address, user, passward).await;
        ftp.list(None).await;
        ftp.download("test.txt","").await;
        ftp.disconnect().await;
    }
    async fn async_ftp_download() {
        fs::remove_file("test.txt").unwrap();
        let address = "192.168.1.4:21";
        let user = "God";
        let passward = "52531225253.";
        let mut ftp = FTP::login(address, user, passward).await;
        ftp.download("test.txt","").await;
        ftp.disconnect().await;
        let dir = fs::read_dir("").unwrap();
        let find = dir.into_iter().find(|x|{x.as_ref().unwrap().file_name()=="test.txt"});
        assert!(find.is_some());
    }


    #[test]
    fn test_ftp_list(){
        BLOCK!(async_ftp_list());
    }
    #[test]
    fn test_ftp_download(){
        BLOCK!(async_ftp_download())
    }
    #[test]
    fn test_ftp_login(){
        BLOCK!(async_ftp_login())
    }
}
