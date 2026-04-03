use quick_xml::events::attributes::AttrError;

#[derive(Debug)]
pub enum ParseError {
    Xml(quick_xml::Error),
    Attr(AttrError),
    MissingName { tag: String },
    Full(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(err) => write!(f, "{err}"),
            Self::Attr(err) => write!(f, "{err}"),
            Self::MissingName { tag } => {
                write!(f, "missing required Name attribute on <{tag}>")
            }
            Self::Full(msg) => write!(f, "full parser: {msg}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Xml(err) => Some(err),
            Self::Attr(err) => Some(err),
            Self::MissingName { .. } | Self::Full(_) => None,
        }
    }
}

impl From<quick_xml::Error> for ParseError {
    fn from(value: quick_xml::Error) -> Self {
        Self::Xml(value)
    }
}

impl From<AttrError> for ParseError {
    fn from(value: AttrError) -> Self {
        Self::Attr(value)
    }
}
