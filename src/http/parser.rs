#![allow(dead_code)]

use std::fs::File;
use clap::{App, Arg};
use std::io;
use std::io::BufRead;
use std::path::Path;

/// 命令行参数,保存用户输入的各个参数
pub struct CommandArgument {
    url:Vec<String>,//保存多个url链接
    out_path:Option<String>,//保存路径
    concurrency:Option<u16>,
}

impl CommandArgument {
    pub fn new() -> Self {
        Self{
            url:Vec::new(),
            out_path:None,
            concurrency:None,
        }
    }
    /// 解析命令行参数如果出现参数缺失将会返回相关错误信息
    pub fn parse(&mut self)->Result<(),&str>{
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::new("url")
                    .short('u')
                    .long("url")
                    .help("download url")
                    .multiple_values(true)
                    .takes_value(true),
            )
            .arg(
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("the filename include urls")
                    .takes_value(true)
            )
            .arg(Arg::new("output")
                .help("output path")
                .long("output")
                .default_missing_value(".")
                .short('o')
                .takes_value(true)
            )
            .arg(
                Arg::new("concurrency")
                    .short('c')
                    .long("concurrency")
                    .help("download threads")
                    .takes_value(true),
            )
            .get_matches();
        match matcher.value_of("concurrency") {
            None => self.concurrency = Some(8),
            Some(val) => self.concurrency = Some(val.parse::<u16>().unwrap())
        }
        match matcher.values_of("url") {
            None => {}
            Some(val) => {
                let urls :Vec<&str> = val.collect();
                urls.into_iter().for_each(|url|self.url.push(url.to_string()));
            }
        }
        match matcher.value_of("output") {
            None => self.out_path = Some(".".to_string()),
            Some(val) => self.out_path = Some(val.to_string())
        }
        match matcher.value_of("input") {
            None => {}
            Some(filepath) => {
                let mut urls = self.get_url_from_file(filepath);
                self.url.append(&mut urls);
            }
        }
        if self.url.len()==0{
            return Err("please input url");
        }
        Ok(())
    }
    /// 从文件中解析url 链接
    fn get_url_from_file(&self,file_path:&str)->Vec<String>{
        fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
            where P: AsRef<Path>, {
            let file = File::open(filename).unwrap();
            Ok(io::BufReader::new(file).lines())
        }
        let mut urls = Vec::new();
        if let Ok(lines) = read_lines(file_path){
            for line in lines {
                if let Ok(url) = line {
                    urls.push(url)
                }
            }
        }
        urls
    }

    /// 获取url
    pub fn get_url(&self) -> Vec<String> {
        self.url.clone()
    }
    /// 获取线程数量
    pub fn get_concurrency(&self) -> Option<u16>{
        self.concurrency.clone()
    }
    /// 获取保存路径
    pub fn get_output_path(&self) -> Option<String>{
        self.out_path.clone()
    }
}


#[cfg(test)]
mod parse_test {
    use clap::{App, Arg};
    use super::CommandArgument;

    #[test]
    fn test_command_parser_unparse() {
        let parser = CommandArgument::new();
        assert_eq!(parser.url.len(), 0);
        assert_eq!(parser.get_concurrency(), None);
        assert_eq!(parser.out_path,None);
        assert_eq!(parser.concurrency,None)
    }
    #[test]
    fn test_get_url_from_file(){
        let command = CommandArgument::new();
        let urls = command.get_url_from_file("src/http/urls.txt");
        assert_eq!(urls.len(),3);
    }
    #[test]
    fn test_command_correct(){
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::new("url")
                    .short('u')
                    .long("url")
                    .help("download url")
                    .multiple_values(true)
                    .takes_value(true),
            )
            .arg(
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("the filename include urls")
                    .takes_value(true)
            )
            .arg(Arg::new("output")
                .help("output path")
                .long("output")
                .short('o')
                .default_missing_value(".")
                .takes_value(true)
            )
            .arg(
                Arg::new("concurrency")
                    .short('c')
                    .long("concurrency")
                    .help("download threads")
                    .takes_value(true),
            ).get_matches_from(
            vec!["commandParser","-u","123","456","-i","src/http/urls.txt"]
        );
        let urls:Vec<&str> = matcher.values_of("url").unwrap().collect();
        let input = matcher.value_of("input").unwrap();
        assert_eq!(urls,["123","456"]);
        assert_eq!(input,"src/http/urls.txt");
    }

}