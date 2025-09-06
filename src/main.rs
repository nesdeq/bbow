mod browser;
mod client;
mod extractor;
mod history;
mod links;
mod openai;
mod ui;

use anyhow::Result;
use browser::Browser;
use clap::Parser;

#[derive(Parser)]
#[command(name = "bbow", about = "A CLI browser with AI-powered summaries")]
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

    browser.run().await
}
