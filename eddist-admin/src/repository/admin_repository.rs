use uuid::Uuid;

pub trait AdminRepository: Send + Sync {
    async fn get_roles_by_user_id(&self, user_id: Uuid) -> Vec<String>;
}
