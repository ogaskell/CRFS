mod storage;
mod networking;
mod conflict_res;
mod core;
mod errors;
mod types;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Location of the global config file. Defaults to ~/.config/crfs/config.json
    #[arg(short)]
    global_conf: Option<PathBuf>,
}

#[derive(Subcommand, PartialEq, Eq)]
enum Commands {
    /// Create global config.
    Init,
    /// Create and set up a replica.
    Setup {
        /// Hostname of the remote server to use.
        #[arg(short)]
        server: std::net::SocketAddr,
        /// User UUID. If omitted, a new user will be created.
        #[arg(short)]
        user_id: Option<Uuid>,
        /// Filesystem UUID. If omitted, a new FS will be created.
        #[arg(short)]
        fs_id: Option<Uuid>,
        /// User's name. Optional
        /// If the user exists, this will overwrite any existing name.
        #[arg(long)]
        user_name: Option<String>,
        /// Name for the filesystem. Optional
        /// If the filesystem exists, this will overwrite any existing name.
        #[arg(long)]
        fs_name: Option<String>,
        /// Replica directory. Defaults to the current directory.
        #[arg(short)]
        dir: Option<PathBuf>,
    },
    /// Synchronise a replica with the server.
    Sync {
        /// Replica directory. Defaults to the current directory.
        #[arg(short)]
        dir: Option<PathBuf>
    },
    /// Write out all drivers, to ensure files are of "canonical" form.
    Canonize {
        /// Replica directory. Defaults to the current directory.
        #[arg(short)]
        dir: Option<PathBuf>
    }
}

fn main() {
    let cli = Cli::parse();

    let conf_path = match cli.global_conf {
        None => core::GlobalConfig::get_conf_path(),
        Some(p) => p,
    };

    if cli.command == Commands::Init {return core::init(&conf_path).expect("Initialisation Error.");}

    let mut conf = core::GlobalConfig::read(&conf_path).expect("Error reading global config. Please run the init command first.");

    match &cli.command {
        Commands::Setup {server, user_id, fs_id, user_name, fs_name, dir} => {
            core::setup(&mut conf, &conf_path, server, user_id, fs_id, user_name, fs_name, dir);
        },
        Commands::Sync {dir} => core::sync(conf, dir),
        Commands::Canonize {dir} => core::canonize(conf, dir),
        _ => {panic!();}
    }
}
