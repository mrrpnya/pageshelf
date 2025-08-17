use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    NotFound,
    Corrupted,
    ProviderError,
}

/// A path to an Asset within a Page.
/// This is its own struct as it allows better validation.
#[derive(Debug, PartialEq, Eq)]
pub struct AssetPath {
    // TODO: Needs optimization
    path: String,
}

impl AssetPath {
    pub fn name(&self) -> &str {
        self.path.split('/').last().unwrap()
    }

    pub fn directory(&self) -> String {
        let split = self.path.split('/');
        let count = split.clone().count() - 1;
        split.take(count).collect()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn eq_str(&self, s: &str) -> bool {
        let p = match Self::from_str(s) {
            Ok(v) => v,
            Err(_) => {
                return false 
            }
        };

        self == &p
    }
}

impl FromStr for AssetPath {
    type Err = AssetError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut path = value.to_string();
        path = path.replace("\\", "/");
        if path.starts_with('/') {
            path = path.replacen("/", "", 1);
        }

        Ok(Self { path })
    }
}

impl<'a> ToString for AssetPath {
    fn to_string(&self) -> String {
        self.path.clone()
    }
}

/// Represents a file in a page.
pub trait Asset {
    fn mime_type(&self) -> Option<&str> {
        None
    }
    fn body(&self) -> String;
    fn bytes(&self) -> impl Iterator<Item = u8>;
    fn hash_sha256(&self) -> [u8; 32] {
        // TODO: Calculate SHA256 from .bytes()
        [0; 32]
    }
}

pub trait AssetQueryable {
    async fn asset_at(&self, path: &AssetPath) -> Result<impl Asset, AssetError>;
    fn assets(&self) -> Result<impl Iterator<Item = impl Asset>, AssetError>;
    fn total_bytes(&self) -> Option<u32> {
        None
    }
}

pub trait AssetWritable {
    fn write_asset(&mut self, path: &AssetPath, asset: &impl Asset) -> Result<(), AssetError>;
    fn delete_asset(&mut self, path: &AssetPath) -> Result<(), AssetError>;
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_path_leading_slash() {
        let path_a_1 = AssetPath::from_str("my/file").unwrap();
        let path_a_2 = AssetPath::from_str("/my/file").unwrap();
        assert_eq!(path_a_1, path_a_2)
    }
}