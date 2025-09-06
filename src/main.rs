mod browser;
mod client;
mod extractor;
mod history;
mod jony;
mod links;
mod markdown;
mod openai;
mod ui;
mod ui_common;

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

const AVAILABLE_UIS: &[(&str, &str)] = &[
    ("default", "Original terminal UI with borders and colors"),
    ("jony", "Minimalist Jony Ive-inspired UI"),
];

fn create_ui(ui_name: &str) -> Result<Box<dyn UIInterface>> {
    match ui_name {
        "default" => Ok(Box::new(ui::UI::new()?)),
        "jony" => Ok(Box::new(JonyUI::new()?)),
        _ => {
            let available: Vec<String> = AVAILABLE_UIS
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect();
            Err(anyhow!(
                "Unknown UI: {}. Available options: {}",
                ui_name,
                available.join(", ")
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Validate UI selection
    if !AVAILABLE_UIS.iter().any(|(name, _)| *name == args.ui) {
        eprintln!("Error: Unknown UI '{}'. Available options:", args.ui);
        for (name, desc) in AVAILABLE_UIS {
            eprintln!("  {:<8} - {}", name, desc);
        }
        std::process::exit(1);
    }

    println!("🎨 Using '{}' UI theme", args.ui);

    let ui = create_ui(&args.ui)?;
    let mut browser = Browser::new(ui)?;

    if let Some(url) = args.url {
        browser.navigate(&url).await?;
    }

    browser.run().await
}
