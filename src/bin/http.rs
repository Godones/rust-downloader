use std::time::Instant;
use lib::http::execute;
#[tokio::main]
async fn main() {
    let begin = Instant::now();
    execute().await;
    let end = begin.elapsed().as_millis();
    println!("cost:{}",end);

}
