//! This crate implements an iCalendar server serving Karlsruhe's garbage collection dates as events.
//! It also implements a CLI to just get a single iCalendar file.
//!
//! The dates are read from <https://web6.karlsruhe.de/service/abfall/akal/akal.php>.

pub use ical;

pub mod garbage_client;
