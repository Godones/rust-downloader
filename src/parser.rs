#![allow(dead_code)]
use clap::{App, Arg};

/// 命令行参数,保存用户输入的各个参数
pub struct CommandArgument {
    url: Option<String>,
    output_path: Option<String>,
    concurrency: Option<u16>,
}

impl CommandArgument {
    pub fn new() -> Self {
        Self {
            url: None,
            output_path: None,
            concurrency: None,
        }
    }
    pub fn parse(&mut self) {
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::with_name("url")
                    .short("u")
                    .long("url")
                    .help("download url")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("concurrency")
                    .short("c")
                    .long("concurrency")
                    .help("download threads")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("output")
                    .long("output")
                    .short("o")
                    .help("the saving path")
                    .takes_value(true),
            )
            .get_matches();
        match matcher.value_of("concurrency") {
            None => self.concurrency = Some(8),
            Some(val) => self.concurrency = Some(val.parse::<u16>().unwrap()),
        }
        match matcher.value_of("url") {
            None => panic!("please input url"),
            Some(val) => self.url = Some(String::from(val)),
        }
        match matcher.value_of("output") {
            None => self.output_path = Some(".".to_string()),
            Some(val) => self.output_path = Some(String::from(val)),
        }
    }
    /// 获取url
    pub fn get_url(&self) -> Option<String> {
        self.url.clone()
    }
    /// 获取线程数量
    pub fn get_concurrency(&self) -> Option<u16> {
        self.concurrency
    }
    /// 获取保存路径
    pub fn get_output_path(&self) -> Option<String> {
        self.output_path.clone()
    }
}

#[test]
fn test_command_parser_unparse() {
    let parser = CommandArgument::new();
    assert_eq!(parser.url, None);
    assert_eq!(parser.get_concurrency(), None);
}
