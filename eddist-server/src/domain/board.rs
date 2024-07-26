use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub local_rule: String,
    pub default_name: String,
}
