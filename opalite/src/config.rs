use std::{ collections::HashMap, fs::File, path::PathBuf };
use failure::Error;
use ron;
use crate::{ ShaderKey };


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub title: String,
    pub window_dimensions: (u32, u32),
    pub shaders: HashMap<ShaderKey, PathBuf>,
    pub map_dimensions: (i32, i32, i32),
    pub resources: Vec<PathBuf>,
    pub fonts: Vec<PathBuf>,
    pub font_resolution: u32,
}

impl Config {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let file = File::open(&path)?;
        ron::de::from_reader(file).map_err(|e| e.into())
    }

    pub fn from_str(string: &str) -> Result<Self, Error> {
        ron::de::from_str(string).map_err(|e| e.into())
    }

    pub fn merge(mut self, other: ConfigBuilder) -> Config {
        if let Some(title) = other.title {
            self.title = title;
        }

        if let Some(window_dimensions) = other.window_dimensions {
            self.window_dimensions = window_dimensions;
        }

        if let Some(shaders) = other.shaders {
            for (key, value) in shaders.into_iter() {
                self.shaders.insert(key, value);
            }
        }

        if let Some(map_dimensions) = other.map_dimensions {
            self.map_dimensions = map_dimensions;
        }

        if let Some(resources) = other.resources {
            let mut resources = resources.clone();
            resources.extend(self.resources);
            self.resources = resources;
        }

        if let Some(fonts) = other.fonts {
            self.fonts.extend(fonts);
        }

        if let Some(font_resolution) = other.font_resolution {
            self.font_resolution = font_resolution;
        }

        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigBuilder {
    pub title: Option<String>,
    pub window_dimensions: Option<(u32, u32)>,
    pub shaders: Option<HashMap<ShaderKey, PathBuf>>,
    pub map_dimensions: Option<(i32, i32, i32)>,
    pub resources: Option<Vec<PathBuf>>,
    pub fonts: Option<Vec<PathBuf>>,
    pub font_resolution: Option<u32>,
}

impl ConfigBuilder {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path: PathBuf = path.into();
        let file = File::open(&path)?;
        ron::de::from_reader(file).map_err(|e| e.into())
    }
}
