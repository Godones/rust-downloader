
use lib::bittorrent::execute;
#[tokio::main]
async fn main(){
    execute().await;
    std::process::exit(0);
}