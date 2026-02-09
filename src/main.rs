mod features;

use clap::Parser;
use dotenv::dotenv;

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

    let final_answer: String = features::llm::chat(&args.prompt).await?;
    println!("{}", final_answer);

    Ok(())
}
