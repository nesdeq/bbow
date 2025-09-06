mod browser;
mod client;
mod extractor;
mod history;
mod jony;
mod links;
mod openai;
mod ui;

use anyhow::{anyhow, Result};
use browser::Browser;
use clap::Parser;

// Import UI traits and implementations
use jony::JonyUI;
use ui::UIInterface;

#[derive(Parser)]
#[command(name = "bbow", about = "A CLI browser with AI-powered summaries")]
struct Args {
    #[arg(help = "Initial URL to visit")]
    url: Option<String>,

    #[arg(long, help = "UI theme to use", default_value = "default")]
    ui: String,
}

fn create_ui(ui_name: &str) -> Result<Box<dyn UIInterface>> {
    match ui_name {
        "default" => {
            let ui = ui::UI::new()?;
            Ok(Box::new(ui))
        }
        "jony" => {
            let ui = JonyUI::new()?;
            Ok(Box::new(ui))
        }
        _ => Err(anyhow!(
            "Unknown UI: {}. Available options: default, jony",
            ui_name
        )),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Print available UI options if invalid UI is specified
    match args.ui.as_str() {
        "default" | "jony" => {}
        _ => {
            eprintln!("Error: Unknown UI '{}'. Available options:", args.ui);
            eprintln!("  default - Original terminal UI with borders and colors");
            eprintln!("  jony    - Minimalist Jony Ive-inspired UI");
            std::process::exit(1);
        }
    }

    println!("🎨 Using '{}' UI theme", args.ui);

    let ui = create_ui(&args.ui)?;
    let mut browser = Browser::new(ui)?;

    if let Some(url) = args.url {
        browser.navigate(&url).await?;
    }

    browser.run().await
}
