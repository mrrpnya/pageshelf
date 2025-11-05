//! Page Source factories offer a way of manipulating the output of a Page Source,
//! or efficiently instantiating multiple Page Sources.

use crate::project::source::ProjectSource;

/// Offers an impl-agnostic of creating Page Sources.
pub trait ProjectSourceBuilder: Clone {
    type Source: ProjectSource;

    fn wrap<L: ProjectSourceLayer<Self::Source>>(
        self,
        layer: L,
    ) -> ProjectSourceBuilderLayer<Self, L> {
        ProjectSourceBuilderLayer {
            parent: self,
            layer,
        }
    }

    fn build(&self) -> Self::Source;
}

/// Layers over a Page Source and can modify it.
/// You could, for instance, create a blacklist that won't accept certain queries.
pub trait ProjectSourceLayer<PS: ProjectSource>: Clone {
    type Source: ProjectSource;

    fn wrap(&self, page_source: PS) -> Self::Source;
}

#[derive(Clone)]
pub struct ProjectSourceBuilderLayer<F: ProjectSourceBuilder, L: ProjectSourceLayer<F::Source>> {
    parent: F,
    layer: L,
}

impl<F: ProjectSourceBuilder, L: ProjectSourceLayer<F::Source>> ProjectSourceBuilder
    for ProjectSourceBuilderLayer<F, L>
{
    type Source = L::Source;

    fn build(&self) -> Self::Source {
        let built = self.parent.build();

        self.layer.wrap(built)
    }
}
