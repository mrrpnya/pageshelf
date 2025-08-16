use crate::asset::{AssetQueryable, AssetWritable};

pub mod memory;

pub trait Cache: AssetQueryable + AssetWritable {
    
}