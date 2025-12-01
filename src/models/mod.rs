pub mod coordinate;
pub mod version;

pub use coordinate::{CoordinateError, MavenCoordinate};
pub use version::{MavenVersion, UpdateType, VersionStability};
