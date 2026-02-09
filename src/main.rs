mod features;

use clap::Parser;
use dotenv::dotenv;
use serde_json::Value;

use features::tools::ResponseOutput;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Args::parse();

    eprintln!("Logs from your program will appear here!");

    let response: Value = features::llm::chat(&args.prompt).await?;

    if let Some(output) = features::tools::execute_tool_call(&response) {
        match output {
            ResponseOutput::FileContents(s) => print!("{}", s),
            ResponseOutput::Text(s) => println!("{}", s),
        }
    }

    Ok(())
}
