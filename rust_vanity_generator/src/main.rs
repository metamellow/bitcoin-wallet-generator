use anyhow::Result;
use clap::Parser;

mod bitcoin;
mod evm;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target words to search for (comma-separated)
    #[arg(short, long)]
    target_words: Option<String>,

    /// Number of CPU threads to use
    #[arg(short, long)]
    threads: Option<usize>,

    /// Chain to use (btc or evm)
    #[arg(short, long, default_value = "btc")]
    chain: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.chain.to_lowercase().as_str() {
        "btc" => {
            println!("Starting Bitcoin address generation...");
            bitcoin::generate_btc_address(args.target_words, args.threads)?;
        }
        "evm" => {
            println!("Starting EVM address generation...");
            evm::generate_evm_address()?;
        }
        _ => {
            println!("Invalid chain option. Use 'btc' or 'evm'");
            return Ok(());
        }
    }

    Ok(())
}
