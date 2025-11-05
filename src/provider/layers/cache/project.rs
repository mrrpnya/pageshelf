use std::sync::Arc;

use crate::{
    Cache,
    project::{Project, ProjectOwner},
    provider::layers::cache::{owner::CacheProjectOwner, page::CachePage},
};

pub struct CacheProject<'a, P: Project, C: Cache> {
    pub upstream: P,
    pub owner: &'a str,
    pub cache: Arc<C>,
}

impl<'a, P: Project, C: Cache + 'static> Project for CacheProject<'a, P, C> {
    type Page<'b>
        = CachePage<P::Page<'b>, C>
    where
        Self: 'b;

    type Error = P::Error;

    fn name(&self) -> &str {
        self.upstream.name()
    }

    async fn channels<'b>(
        &'b self,
    ) -> Result<impl Iterator<Item = Self::Page<'b>> + 'b, Self::Error> {
        self.upstream.channels().await.map(|i| {
            i.map(|p| CachePage::new(p, self.cache.clone(), self.owner.to_string(), self.name()))
        })
    }

    fn default_channel(&self) -> Option<&str> {
        self.upstream.default_channel()
    }
}
