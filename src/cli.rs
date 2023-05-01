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
    /// Returns the formatted address for the application to run on.
    ///
    /// # Returns
    ///
    /// `<HOST>:<PORT>`
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use super::*;

    #[rstest]
    fn test_args_address(
        #[values("0.0.0.0", "192.168.0.12", "127.0.0.55")] host: String,
        #[values(u16::MIN, u16::MAX, 8080, 12345)] port: u16,
    ) {
        let args = Args {
            host: host.clone(),
            port,
        };
        assert_eq!(args.address(), format!("{}:{}", host, port))
    }
}
