use std::sync::Arc;

use crate::{Cache, project::ProjectOwner, provider::layers::cache::project::CacheProject};

pub struct CacheProjectOwner<O: ProjectOwner, C: Cache> {
    pub upstream: O,
    pub cache: Arc<C>,
}

impl<O: ProjectOwner, C: Cache + 'static> ProjectOwner for CacheProjectOwner<O, C> {
    type Project<'a>
        = CacheProject<'a, O::Project<'a>, C>
    where
        Self: 'a;

    fn name(&self) -> &str {
        self.upstream.name()
    }

    async fn projects<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Project<'a>> + 'a, crate::project::ProjectError> {
        self.upstream.projects().await.map(move |i| {
            i.map(move |p| CacheProject {
                upstream: p,
                owner: self.name(),
                cache: self.cache.clone(),
            })
        })
    }
}
