//! Command line parsing.

use clap::Parser;

/// Command line arguments.
#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Address to run the application on.
    pub host: String,
    /// Port number to run the application on.
    pub port: u16,
}

impl Args {
    ///
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
