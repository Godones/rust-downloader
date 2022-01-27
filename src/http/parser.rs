#![allow(dead_code)]

use clap::{App, Arg};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use regex::Regex;

/// 命令行参数,保存用户输入的各个参数
pub struct CommandArgument {
    url: Vec<String>,         //保存多个url链接
    out_path: Option<String>, //保存路径
    concurrency: Option<u16>,
}

impl CommandArgument {
    pub fn new() -> Self {
        Self {
            url: Vec::new(),
            out_path: None,
            concurrency: None,
        }
    }
    /// 解析命令行参数如果出现参数缺失将会返回相关错误信息
    pub fn parse(&mut self) -> Result<(), &str> {
        let url_help = r#"
        url链接，允许多个url出现
        支持正则表达式生成，只需在url中插入[]即可
        目前正则表达式支持下列格式:
        [[0-2]]:表示此位置可以为数字0,1,2
        [[a-x]]:表示此位置可以为字母a,b...x
        "#;
        let matcher = App::new("commandParser")
            .version("0.1")
            .author("chenlinfeng")
            .about("help parse the commands")
            .arg(
                Arg::new("url")
                    .short('u')
                    .long("url")
                    .help(url_help)
                    .multiple_values(true)
                    .takes_value(true),
            )
            .arg(
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("包含url的文件名称")
                    .takes_value(true),
            )
            .arg(
                Arg::new("output")
                    .help("输出路径")
                    .long("output")
                    .default_missing_value(".")
                    .short('o')
                    .takes_value(true),
            )
            .arg(
                Arg::new("concurrency")
                    .short('c')
                    .long("concurrency")
                    .help("下载线程数")
                    .takes_value(true),
            )
            .get_matches();
        match matcher.value_of("concurrency") {
            None => self.concurrency = Some(8),
            Some(val) => self.concurrency = Some(val.parse::<u16>().unwrap()),
        }
        match matcher.values_of("url") {
            None => {}
            Some(val) => {
                let urls: Vec<&str> = val.collect();
                urls.into_iter()
                    .for_each(|url| {
                        if let Ok(mut val) = self.re_for_url(url){
                            self.url.append(&mut val)
                        }
                    });
            }
        }
        match matcher.value_of("output") {
            None => self.out_path = Some(".".to_string()),
            Some(val) => self.out_path = Some(val.to_string()),
        }
        match matcher.value_of("input") {
            None => {}
            Some(filepath) => {
                let mut urls = self.get_url_from_file(filepath);
                self.url.append(&mut urls);
            }
        }
        if self.url.len() == 0 {
            return Err("please input url");
        }
        Ok(())
    }
    /// 从文件中解析url 链接
    fn get_url_from_file(&self, file_path: &str) -> Vec<String> {
        fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
        where
            P: AsRef<Path>,
        {
            let file = File::open(filename).unwrap();
            Ok(io::BufReader::new(file).lines())
        }
        let mut urls = Vec::new();
        if let Ok(lines) = read_lines(file_path) {
            for line in lines {
                if let Ok(url) = line {
                    if let Ok(mut val) = self.re_for_url(url.as_str()){
                        urls.append(&mut val)
                    }
                }
            }
        }
        urls
    }
    /// 检查url链接中的正则表达式
    fn re_for_url(&self,url:&str)->Result<Vec<String>,&str>{
        let regex1 = Regex::new(r"(\[\[.{3,}?\]\])").unwrap();
        let mut urls = Vec::new();
        let answer = regex1.captures_iter(url);
        let mut flag = false;
        for it in answer{
            // 匹配到的部分
            flag = true;
            let x = &it[0];
            let len = x.len();
            //去掉[[]]
            let need = &x.as_bytes()[2..len-2];
            let need = core::str::from_utf8(need).unwrap();
            //切分
            let (first,second) = need.split_once("-").unwrap();
            if let Ok(small) = first.parse::<usize>(){
                if let Ok(big) = second.parse::<usize>(){
                    if small<=big{
                        for i in small..=big{
                            let source = url.clone();
                            let to = source.replace(x, i.to_string().as_str());
                            urls.push(to);
                        }
                    }
                }
            }
            else{
                //字母
                if first.len()!=1||second.len()!=1{
                    return Err("请检查错误");
                }
                let first = first.as_bytes()[0];
                let second  = second.as_bytes()[0];
                if first>second { return Err("请检查错误");}
                // println!("{:?}-{:?}",first,second);
                for i in first as usize..=second as usize{
                    let source = url.clone();
                    let to = source.replace(x, (i as u8 as char).to_string().as_str());
                    urls.push(to);
                }
            }
        }
        if urls.len()==0&&flag==false{
            //未匹配到任何正则表达式
            urls.push(url.to_string());
            return Ok(urls)
        }
        Ok(urls)
    }
    /// 获取url
    pub fn get_url(&self) -> Vec<String> {
        self.url.clone()
    }
    /// 获取线程数量
    pub fn get_concurrency(&self) -> Option<u16> {
        self.concurrency.clone()
    }
    /// 获取保存路径
    pub fn get_output_path(&self) -> Option<String> {
        self.out_path.clone()
    }
}

#[cfg(test)]
mod parse_test {
    use super::CommandArgument;
    use clap::{App, Arg};

    #[test]
    fn test_command_parser_unparse() {
        let parser = CommandArgument::new();
        assert_eq!(parser.url.len(), 0);
        assert_eq!(parser.get_concurrency(), None);
        assert_eq!(parser.out_path, None);
        assert_eq!(parser.concurrency, None)
    }
    #[test]
    fn test_get_url_from_file() {
        let command = CommandArgument::new();
        let urls = command.get_url_from_file("src/http/urls.txt");
        assert_eq!(urls.len(), 3);
    }
    #[test]
    fn test_command_correct() {
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
                    .takes_value(true),
            )
            .arg(
                Arg::new("output")
                    .help("output path")
                    .long("output")
                    .short('o')
                    .default_missing_value(".")
                    .takes_value(true),
            )
            .arg(
                Arg::new("concurrency")
                    .short('c')
                    .long("concurrency")
                    .help("download threads")
                    .takes_value(true),
            )
            .get_matches_from(vec![
                "commandParser",
                "-u",
                "123",
                "456",
                "-i",
                "src/http/urls.txt",
            ]);
        let urls: Vec<&str> = matcher.values_of("url").unwrap().collect();
        let input = matcher.value_of("input").unwrap();
        assert_eq!(urls, ["123", "456"]);
        assert_eq!(input, "src/http/urls.txt");
    }
    #[test]
    fn test_regex_correct(){
        let test1 = "[[a-c]]";
        let command = CommandArgument::new();
        let a = command.re_for_url(test1).unwrap();
        assert_eq!(a,vec!["a","b","c"]);
        let a = command.re_for_url("www.baidu[[a-e]].exe").unwrap();
        assert_eq!(a.len(),5);
        let a = command.re_for_url("[[1-100]]").unwrap();
        assert_eq!(a.len(),100);
        let a = command.re_for_url("www.[[1-1]]").unwrap();
        assert_eq!(a,vec!["www.1"]);
        let test1 = "[[11-1]]";
        let command = CommandArgument::new();
        let a = command.re_for_url(test1).unwrap();
        assert_eq!(a.len(),0);
        let a = command.re_for_url("1222").unwrap();
        assert_eq!(a,vec!["1222"]);
    }
    #[test]
    #[should_panic]
    fn test_regex_fail(){
        let test1 = "[[aa-c]]";
        let command = CommandArgument::new();
        let _ = command.re_for_url(test1).unwrap();
    }

}
