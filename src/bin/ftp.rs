use lib::ftp::execute;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    execute().await;
    Ok(())
}
