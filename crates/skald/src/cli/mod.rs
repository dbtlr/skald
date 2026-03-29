pub mod aliases;
pub mod commit;
pub mod completions;
pub mod config;
pub mod doctor;
pub mod root;

pub use root::{Cli, Command, ConfigAction};
