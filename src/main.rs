mod browser;
mod client;
mod extractor;
mod openai;
mod links;
mod ui;
mod history;

use anyhow::Result;
use clap::Parser;
use browser::Browser;

#[derive(Parser)]
#[command(name = "bbow")]
#[command(about = "A CLI browser with AI-powered summaries")]
struct Args {
    #[arg(help = "Initial URL to visit")]
    url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let mut browser = Browser::new()?;
    
    if let Some(url) = args.url {
        browser.navigate(&url).await?;
    }
    
    browser.run().await?;
    
    Ok(())
}