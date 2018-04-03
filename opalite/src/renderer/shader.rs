use std::{ fs::File, io::Read, path::PathBuf };
use failure::{ self, Error };
use glsl_to_spirv::{ self, ShaderType };
use crate::{ Config, Resources, RLock };

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ShaderKey(String);

impl ShaderKey {
    pub fn new<S: Into<String>>(name: S) -> Self {
        ShaderKey(name.into())
    }
}

#[derive(Clone)]
pub struct Shader {
    pub vertex: Vec<u8>,
    pub fragment: Vec<u8>,
}

impl Shader {
    pub fn load(path: &PathBuf, resources: &RLock<Resources>) -> Result<Self, Error> {
        let resources = resources.read().unwrap();

        let read_to_vec = |file: &mut File| -> Result<Vec<u8>, Error> {
            let mut code = vec![];
            file.read_to_end(&mut code)?;
            Ok(code)
        };

        let compile_shader = |mut path: PathBuf, shader_type: ShaderType| -> Result<Vec<u8>, Error> {
            let filename = match path.file_name() {
                Some(name) => match name.to_str() {
                    Some(name) => name,
                    None => bail!("Shader Filename is invalid."),
                },
                None => bail!("Shader Filename is invalid."),
            };

            let ext = match shader_type {
                ShaderType::Vertex => "vert",
                ShaderType::Fragment => "frag",
                _ => bail!("Unsupported Shader Type"),
            };

            path.set_file_name(format!("{}.{}.glsl", filename, ext));
            let file = resources.get_string(&path)?;
            let mut file = glsl_to_spirv::compile(&file, shader_type)
                .map_err(|e| failure::err_msg(e))?;
            read_to_vec(&mut file)
        };

        let vertex = compile_shader(path.clone(), ShaderType::Vertex)?;
        let fragment = compile_shader(path.clone(), ShaderType::Fragment)?;

        Ok(Self { vertex, fragment })
    }

    pub fn load_from_config(config: &Config, resources: &RLock<Resources>, shader: &ShaderKey) -> Result<Shader, Error> {
        ensure!(config.shaders.contains_key(&shader), "Shader isn't in Opal.ron");

        let path = config.shaders.get(&shader).unwrap();
        Shader::load(&path, resources)
    }
}
