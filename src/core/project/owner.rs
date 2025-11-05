use crate::project::{Project, ProjectError};

pub trait ProjectOwner: Send {
    type Project<'a>: Project + 'a
    where
        Self: 'a;

    fn name(&self) -> &str;
    async fn projects<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Project<'a>> + 'a, ProjectError>;
    async fn get_project<'a>(
        &'a self,
        name: &str,
    ) -> Result<Option<Self::Project<'a>>, ProjectError> {
        self.projects()
            .await
            .map(|i| i.filter(|f| f.name() == name).next())
    }
}
