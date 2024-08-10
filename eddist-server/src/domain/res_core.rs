use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct ResCore<'a> {
    pub from: &'a str,
    pub mail: &'a str,
    pub body: Cow<'a, str>,
}
