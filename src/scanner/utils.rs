pub fn to_lowercase_vec(data: &[impl AsRef<str>]) -> Vec<String> {
    data.iter().map(|s| s.as_ref().to_lowercase()).collect()
}
