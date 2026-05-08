use mamut_params::MacroId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod model;
pub use model::*;

mod errors;
pub use errors::*;

mod serde_io;
pub use serde_io::*;

mod validation;
pub use validation::*;

#[cfg(test)]
mod tests;
