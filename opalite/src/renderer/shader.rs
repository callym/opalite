#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ShaderKey(String);

impl ShaderKey {
    pub fn new(name: String) -> Self {
        ShaderKey(name)
    }
}
