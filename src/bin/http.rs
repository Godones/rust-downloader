use lib::http::download;
#[tokio::main]
async fn main() {
    download().await;
}