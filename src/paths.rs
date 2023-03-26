/// Def ines a test path.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TestPath {
    /// The path to test, not including the domain.
    pub path: String,
    /// The name of this path, for use in metrics.
    pub name: String,
}

impl TestPath {
    pub fn new(path: &str, name: &str) -> TestPath {
        TestPath {
            path: String::from(path),
            name: String::from(name),
        }
    }
}
