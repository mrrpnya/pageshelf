//! Page Source factories offer a way of manipulating the output of a Page Source,
//! or efficiently instantiating multiple Page Sources.

use super::PageSource;

/// Offers an impl-agnostic of creating Page Sources.
pub trait PageSourceFactory: Clone {
    type Source: PageSource;

    fn wrap<L: PageSourceLayer<Self::Source>>(self, layer: L) -> PageSourceFactoryLayer<Self, L> {
        PageSourceFactoryLayer {
            parent: self,
            layer,
        }
    }

    fn build(&self) -> Result<Self::Source, ()>;
}

/// Layers over a Page Source and can modify it.
/// You could, for instance, create a blacklist that won't accept certain queries.
pub trait PageSourceLayer<PS: PageSource>: Clone {
    type Source: PageSource;

    fn wrap(&self, page_source: PS) -> Self::Source;
}

#[derive(Clone)]
pub struct PageSourceFactoryLayer<F: PageSourceFactory, L: PageSourceLayer<F::Source>> {
    parent: F,
    layer: L,
}

impl<F: PageSourceFactory, L: PageSourceLayer<F::Source>> PageSourceFactory
    for PageSourceFactoryLayer<F, L>
{
    type Source = L::Source;

    fn build(&self) -> Result<Self::Source, ()> {
        let built = match self.parent.build() {
            Ok(v) => v,
            Err(_) => {
                return Err(());
            }
        };

        Ok(self.layer.wrap(built))
    }
}
