pub mod aliases;
pub mod commit;
pub mod completions;
pub mod config;
pub mod doctor;
#[cfg(feature = "integrations")]
pub mod integrations;
pub mod pr;
pub mod root;
pub mod upgrade;

pub use root::{AliasAction, Cli, Command, ConfigAction};
#[cfg(feature = "integrations")]
pub use root::IntegrationTarget;
