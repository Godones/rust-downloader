use clap::{App, Arg};

pub struct CommandArgument{
    file_path:Option<String>,
    target_path:String,
}

impl CommandArgument {
    pub fn new()->Self{
        Self{
            file_path:None,
            target_path:"".to_string()
        }
    }
    pub fn parse(&mut self){
        let matcher = App::new("xerus")
            .version("0.1.0")
            .about("A command-line BitTorrent client, written in Rust.")
            .author("chenlinfeng")
            .arg(
                Arg::new("torrent")
                    .short('t')
                    .long("torrent")
                    .help("The path to the torrent")
                    .number_of_values(1)
                    .required(true),
            )
            .arg(
                Arg::new("file")
                    .short('o')
                    .long("output")
                    .help("The path where to save the file")
                    .number_of_values(1)
            )
            .get_matches();
        self.file_path = Some(matcher.value_of("torrent").unwrap().to_string());
        if matcher.value_of("file").is_some(){
            self.target_path = matcher.value_of("file").unwrap().to_string();
        }
    }

}