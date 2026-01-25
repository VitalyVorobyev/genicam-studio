mod error;
mod model;
mod parser;

pub use error::ParseError;
pub use model::{EnumEntry, NumericConstraints, RawNode, UiCategory, UiGraph, UiNode, UiNodeKind};
pub use parser::parse_genicam_xml;
