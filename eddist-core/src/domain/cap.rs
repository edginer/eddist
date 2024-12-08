use sha3::Digest;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Cap {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub password_hash: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub fn calculate_cap_hash(password: &str, salt: &str) -> String {
    let salt = salt.trim();
    let stretch_count = 3;
    let mut hash = format!("{password}{salt}");
    for i in 0..stretch_count {
        let result = format!("{:x}", sha3::Sha3_512::digest(hash.as_bytes()));
        hash = match i % 3 {
            0 => format!("{result}{salt}"),
            1 => format!("{salt}{result}"),
            2 => format!("{salt}{result}{salt}"),
            _ => unreachable!(),
        };
    }
    format!("{:x}", sha3::Sha3_512::digest(hash.as_bytes()))
}
