use clap::Parser;

#[derive(Parser)]
#[command(about = "Git-native workspace manager for isolated development Heads")]
struct Cli {}

fn main() {
    Cli::parse();
}
