/// Describes a test target.
#[derive(Debug, Clone)]
pub struct TestTarget<'a> {
    /// A name for the server under test (e.g. "language/framework").
    pub server_name: &'a str,
    /// The number of CPUs the server is using for this test.
    pub num_cpus: usize,
    /// The amount of RAM, in MB, the server is using for this test.
    pub ram_mb: usize,
    /// Whether or not these tests accept compressed results.
    pub is_compressed: bool,
}

impl<'a> TestTarget<'a> {
    /// Gets the expected docker target name ("hello-<server_name>").
    pub fn docker_target(&self) -> String {
        format!("hello-{}", self.server_name)
    }

    /// Converts this target to a unique name.
    pub fn name(&self) -> String {
        let compression = if self.is_compressed {
            "compressed"
        } else {
            "uncompressed"
        };
        // Docker rejects container names with "/", so convert slashes to "-".
        format!(
            "{}-{}-cpus-{}-ram-{}m",
            self.server_name.replace("/", "-"),
            compression,
            self.num_cpus,
            self.ram_mb
        )
    }
}
