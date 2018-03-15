use std::{ collections::HashMap, fs::File, path::PathBuf };
use failure::Error;
use ron;
use crate::{ ShaderKey };

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShaderLocation {
    Builtin(PathBuf),
    Custom(PathBuf),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub title: String,
    pub window_dimensions: (u32, u32),
    pub shaders: HashMap<ShaderKey, ShaderLocation>,
}

impl Config {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let file = File::open(&path)?;
        ron::de::from_reader(file).map_err(|e| e.into())
    }

    pub fn merge(mut self, other: ConfigBuilder) -> Config {
        if let Some(title) = other.title {
            self.title = title;
        }

        if let Some(dimensions) = other.window_dimensions {
            self.window_dimensions = dimensions;
        }

        if let Some(shaders) = other.shaders {
            for (key, value) in shaders.into_iter() {
                self.shaders.insert(key, ShaderLocation::Custom(value));
            }
        }

        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigBuilder {
    pub title: Option<String>,
    pub window_dimensions: Option<(u32, u32)>,
    pub shaders: Option<HashMap<ShaderKey, PathBuf>>,
}

impl ConfigBuilder {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let file = File::open(&path)?;
        ron::de::from_reader(file).map_err(|e| e.into())
    }
}
