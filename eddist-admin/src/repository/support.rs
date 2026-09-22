pub(crate) fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
