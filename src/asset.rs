#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    NotFound,
    Corrupted,
    ProviderError,
}

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
}

impl TryFrom<&str> for AssetPath {
    type Error = AssetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // TODO: Validation!!!
        let mut path = value.to_string();
        path = path.replace("\\", "/");
        if path.starts_with('/') {
            path = format!("/{}", path);
        }

        Ok(Self { path })
    }
}

impl<'a> ToString for AssetPath {
    fn to_string(&self) -> String {
        self.path.clone()
    }
}

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
