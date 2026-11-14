//! Sans-IO protocol, program model, `.syx` files and simulator for the
//! Sequential Prophet Rev2.

mod curate;
pub mod driver;
pub mod fields;
mod generate;
pub mod msg;
pub mod program;
pub mod session;
pub mod sim;
pub mod syx;

pub use driver::DRIVER;
pub use fields::Field;
pub use oscyl_driver::DriverError as Error;
pub use program::Program;
