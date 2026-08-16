use sha3::Digest;
use uuid::Uuid;

use crate::utils::to_hex;

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
        let result = to_hex(sha3::Sha3_512::digest(hash.as_bytes()));
        hash = match i % 3 {
            0 => format!("{result}{salt}"),
            1 => format!("{salt}{result}"),
            2 => format!("{salt}{result}{salt}"),
            _ => unreachable!(),
        };
    }
    to_hex(sha3::Sha3_512::digest(hash.as_bytes()))
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn known_vector_stable_across_digest_crate_bump() {
        assert_eq!(
            calculate_cap_hash("hunter2password", "somesalt"),
            "b52d20c9e314ad024a31e3c920f0be71224d112d7dd646b2ec9288ab1e16a2c24801220cffffaf99a09e66f9c55e6e8b6b3ad1baa11fa5a43660e90c40d33d46"
        );
    }
}
