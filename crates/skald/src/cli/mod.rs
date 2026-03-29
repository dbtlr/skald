pub mod aliases;
pub mod commit;
pub mod completions;
pub mod config;
pub mod doctor;
pub mod pr;
pub mod root;
pub mod upgrade;

pub use root::{Cli, Command, ConfigAction};
