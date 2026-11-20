use crate::metas::domain::{Meta, MetasError};
use crate::models::Movement;
use uuid::Uuid;

pub trait MetasPort: Send + Sync {
    fn save(&self, meta: Meta) -> Result<Meta, MetasError>;
    fn update(&self, meta: Meta) -> Result<Meta, MetasError>;
    fn get_by_id(&self, id: Uuid) -> Result<Option<Meta>, MetasError>;
    fn list(&self) -> Result<Vec<Meta>, MetasError>;
    fn cancelar(&self, id: Uuid) -> Result<Meta, MetasError>;
    fn reactivar(&self, id: Uuid) -> Result<Meta, MetasError>;
    fn migrate(&self) -> Result<(), MetasError>;
    fn apartados_de_meta(&self, id: Uuid) -> Result<Vec<Movement>, MetasError>;
}
