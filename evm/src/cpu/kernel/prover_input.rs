#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct ProverInputFn(Vec<String>);

impl From<Vec<String>> for ProverInputFn {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}
