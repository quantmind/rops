use crate::{error::RopsResult, settings::Settings, utils};

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ExtraCommand {
    /// Generate a trong cookie-secret for ouath2
    CookieSecret {
        /// Length of the secret to generate
        #[arg(short, long, default_value = "32")]
        length: usize,
    },
}

impl ExtraCommand {
    /// Run the extra command
    pub fn run(&self, _settings: &Settings) -> RopsResult<()> {
        match self {
            Self::CookieSecret { length } => {
                let secret = utils::random_base_64(*length)?;
                println!("Generated cookie secret: {}", secret);
                Ok(())
            }
        }
    }
}
