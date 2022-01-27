use clap::{App, Arg};

pub struct CommandArgument {
    username: Option<String>,
    password: Option<String>,
    address: Option<String>,
    out_path: Option<String>,
    target: Option<(String, String)>,
}

impl CommandArgument {
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            address: None,
            out_path: None,
            target: None,
        }
    }
    /// 解析命令行参数如果出现参数缺失将会返回相关错误信息
    pub fn parse(&mut self) -> Result<(), &str> {
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::new("user")
                    .short('u')
                    .long("user")
                    .help("user name and password")
                    .takes_value(true),
            )
            .arg(
                Arg::new("address")
                    .help("ftp address")
                    .long("address")
                    .short('a')
                    .required(true)
                    .takes_value(true),
            )
            .arg(
                Arg::new("download")
                    .short('o')
                    .long("output")
                    .help("the output path")
                    .default_missing_value("")
                    .takes_value(true),
            )
            .get_matches();

        // println!("{:?}",matcher);
        match matcher.value_of("user") {
            None => {}
            Some(val) => {
                let s: Vec<&str> = val.split(':').collect();
                if s.len() == 2 {
                    self.username = Some(s[0].to_string());
                    self.password = Some(s[1].to_string());
                }
            }
        }
        match matcher.value_of("address") {
            None => {}
            Some(address) => {
                if let Some(_) = address.find('/') {
                    let filename: Vec<&str> = address.split('/').collect();
                    self.address = Some(filename[0].to_string());
                    let target_path = filename[1..filename.len() - 1].join("/");
                    let filename = filename[filename.len() - 1];
                    self.target = Some((target_path, filename.to_string()));
                } else {
                    self.address = Some(address.to_string());
                }
            }
        }
        if self.username.is_none() || self.address.is_none() || self.password.is_none() {
            return Err("please recheck your input");
        }
        match matcher.value_of("download") {
            None => {}
            Some(output) => self.out_path = Some(output.to_string()),
        }
        Ok(())
    }
    /// 获取需要执行的任务
    pub fn get_output(&self) -> Option<String> {
        self.out_path.clone()
    }
    /// 获取url
    pub fn get_username(&self) -> Option<String> {
        self.username.clone()
    }
    /// 获取线程数量
    pub fn get_password(&self) -> Option<String> {
        self.password.clone()
    }
    /// 获取保存路径
    pub fn get_address(&self) -> Option<String> {
        self.address.clone()
    }

    /// 获取目标路径和下载的文件名称
    pub fn get_target_path(&self) -> Option<(String, String)> {
        self.target.clone()
    }
}
#[cfg(test)]
mod ftp_parse_test {
    use clap::{App, Arg};
    #[test]
    fn test_ftp_parse_correct() {
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::new("user")
                    .short('u')
                    .long("user")
                    .help("user name and password")
                    .takes_value(true),
            )
            .arg(
                Arg::new("address")
                    .help("ftp address")
                    .long("address")
                    .short('a')
                    .required(true)
                    .takes_value(true),
            )
            .get_matches_from(vec!["commandParser", "-a", "110.123.25", "-u", "god:12346"]);
    }
}
