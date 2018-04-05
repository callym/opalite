use std::{ env, fs::File, io::{ self, prelude::* }, path::PathBuf };
use failure::Error;
use zip::ZipArchive;
use crate::Config;

#[derive(Debug, Copy, Clone)]
pub enum ResourcesType {
    Zip,
    Folder,
}

#[derive(Clone)]
pub struct Resources {
    archives: Vec<(PathBuf, ResourcesType)>,
}

impl Resources {
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let current_dir = env::current_dir()?;

        let mut resources = PathBuf::new();
        resources.push(current_dir.clone());
        resources.push("resources");
        let mut archives = vec![(resources, ResourcesType::Folder)];

        for resource_path in &config.resources {
            let mut path = PathBuf::new();
            path.push(current_dir.clone());
            path.push(resource_path);

            if !path.exists() {
                let err: io::Error = io::ErrorKind::NotFound.into();
                Err(err)?;
            }

            if path.is_dir() {
                archives.push((path, ResourcesType::Folder));
            } else {
                archives.push((path, ResourcesType::Zip));
            }
        }

        Ok(Self {
            archives,
        })
    }

    pub fn contains<P: Into<PathBuf>>(&self, file_path: P) -> bool {
        let file_path: PathBuf = file_path.into();

        for (path, kind) in &self.archives {
            match kind {
                ResourcesType::Zip => {
                    let mut zip = match File::open(&path) {
                        Ok(zip) => zip,
                        Err(_) => return false,
                    };

                    let mut zip = match ZipArchive::new(zip) {
                        Ok(zip) => zip,
                        Err(_) => return false,
                    };

                    let file_path = file_path.to_str().unwrap().replace("\\", "/");

                    match zip.by_name(&file_path) {
                        Ok(_) => return true,
                        Err(_) => (),
                    }
                },
                ResourcesType::Folder => {
                    let mut full_path = PathBuf::new();
                    full_path.push(path);
                    full_path.push(file_path.clone());

                    if full_path.exists() {
                        return true
                    }
                },
            };
        }

        false
    }

    pub fn get<P: Into<PathBuf>>(&self, file_path: P) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![];
        self.do_reader(file_path, |reader: &mut Read| reader.read_to_end(&mut bytes))?;
        Ok(bytes)
    }

    pub fn get_string<P: Into<PathBuf>>(&self, file_path: P) -> Result<String, Error> {
        let mut string = String::new();
        self.do_reader(file_path, |reader: &mut Read| reader.read_to_string(&mut string))?;
        Ok(string)
    }

    fn do_reader<P: Into<PathBuf>, E: Into<Error>, R>(&self, file_path: P, mut cb: impl FnMut(&mut Read) -> Result<R, E>) -> Result<(), Error> {
        let file_path: PathBuf = file_path.into();

        if self.contains(file_path.clone()) == false {
            let err: io::Error = io::ErrorKind::NotFound.into();
            Err(err)?;
        }

        for (path, kind) in &self.archives {
            match kind {
                ResourcesType::Zip => {
                    let mut zip = File::open(&path)?;
                    let mut zip = ZipArchive::new(zip)?;

                    let file_path = file_path.to_str().unwrap().replace("\\", "/");

                    match zip.by_name(&file_path) {
                        Ok(ref mut file) => return cb(file).map(|_| ()).map_err(|e| e.into()),
                        Err(_) => (),
                    }
                },
                ResourcesType::Folder => {
                    let mut full_path = PathBuf::new();
                    full_path.push(path);
                    full_path.push(file_path.clone());

                    if full_path.exists() {
                        let mut file = File::open(full_path)?;
                        return cb(&mut file).map(|_| ()).map_err(|e| e.into());
                    }
                },
            }
        }

        let err: io::Error = io::ErrorKind::NotFound.into();
        Err(err)?
    }
}
