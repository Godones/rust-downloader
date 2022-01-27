pub mod myftp;
pub mod parser;

use colorful::{Color, Colorful};
use parser::CommandArgument;

pub async fn execute() {
    let mut command = CommandArgument::new();
    if let Ok(()) = command.parse() {
        //如果参数解析正确
        let username = command.get_username().unwrap();
        let password = command.get_password().unwrap();
        let address = command.get_address().unwrap();
        let output = command.get_output().unwrap();
        let target = command.get_target_path().unwrap();
        println!("target: {:?}",target);
        let mut ftp = myftp::FTP::login(&address, &username, &password).await;
        ftp.cwd(target.0.as_str()).await;
        ftp.list(None).await;
        ftp.download(target.1.as_str(), output.as_str()).await;
    } else {
        println!("{}", "Please check your entry".color(Color::Red));
    }
}
