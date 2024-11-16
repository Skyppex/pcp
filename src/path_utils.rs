use std::{
    io::{ErrorKind, Result},
    path::{Path, PathBuf},
};

pub fn convert_to_path(path: Option<&str>) -> Result<Option<PathBuf>> {
    Ok(match path {
        Some(path) => Some(get_path(path)?),
        None => None,
    })
}

pub fn get_path(path: &str) -> Result<PathBuf> {
    let path = match path {
        p if p.starts_with("~") => dirs::home_dir()
            .ok_or(std::io::Error::from(ErrorKind::NotFound))?
            .join(&p[2..]),
        p if p.starts_with("..") => {
            let current_dir = &std::env::current_dir()?;
            let mut current_dir = current_dir
                .parent()
                .ok_or(std::io::Error::from(ErrorKind::NotFound))?;

            let mut path = Path::new(&p[3..]);

            while path.starts_with("..") {
                current_dir = current_dir
                    .parent()
                    .ok_or(std::io::Error::from(ErrorKind::NotFound))?;
                path = Path::new(&path.to_str().unwrap()[3..]);
            }

            current_dir.join(path)
        }
        p if p.starts_with("./") || p.starts_with(".\\") => std::env::current_dir()?.join(&p[2..]),
        p => {
            let path = Path::new(p);
            if !Path::has_root(path) {
                return Ok(std::env::current_dir()?.join(p));
            }

            path.to_path_buf()
        }
    };

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_full_path() {
        let path = "~\\.cargo".to_string();
        let binding = get_path(&path).unwrap();
        let full_path = binding.to_str().unwrap();
        let binding = dirs::home_dir().unwrap().join(".cargo");
        let expected = binding.to_str().unwrap();
        assert_eq!(full_path, expected);
    }

    #[test]
    fn test_get_full_path_from_base_less() {
        let path = ".cargo".to_string();
        let binding = get_path(&path).unwrap();
        let full_path = binding.to_str().unwrap();
        let binding = std::env::current_dir().unwrap().join(".cargo");
        let expected = binding.to_str().unwrap();
        assert_eq!(full_path, expected);
    }

    #[test]
    fn test_get_full_path_absolute() {
        let path = "C:\\Users\\user\\.cargo".to_string();
        let binding = get_path(&path).unwrap();
        let full_path = binding.to_str().unwrap();
        let binding = Path::new("C:\\Users\\user\\.cargo").to_path_buf();
        let expected = binding.to_str().unwrap();
        assert_eq!(full_path, expected);
    }

    #[test]
    fn test_get_double_dottet_path() {
        let path = "..\\src".to_string();
        let binding = get_path(&path).unwrap();
        let full_path = binding.to_str().unwrap();
        let binding = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .join("src");
        let expected = binding.to_str().unwrap();
        assert_eq!(full_path, expected);
    }

    #[test]
    fn test_get_double_dottet_twice_path() {
        let path = "..\\..\\src".to_string();
        let binding = get_path(&path).unwrap();
        let full_path = binding.to_str().unwrap();
        let binding = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("src");
        let expected = binding.to_str().unwrap();
        assert_eq!(full_path, expected);
    }
}
