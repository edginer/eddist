use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::res::{Res, ResState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NgWord {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub trait NgWordRestrictable {
    fn contains_ng_word(&self, ng_words: &[NgWord]) -> bool;
}

impl NgWordRestrictable for str {
    fn contains_ng_word(&self, ng_words: &[NgWord]) -> bool {
        ng_words.iter().any(|ng_word| self.contains(&ng_word.word))
    }
}

impl NgWordRestrictable for String {
    fn contains_ng_word(&self, ng_words: &[NgWord]) -> bool {
        ng_words.iter().any(|ng_word| self.contains(&ng_word.word))
    }
}

impl<T: ResState> NgWordRestrictable for Res<T> {
    fn contains_ng_word(&self, ng_words: &[NgWord]) -> bool {
        ng_words.iter().any(|ng_word| {
            self.body().contains(&ng_word.word)
                || self.mail().contains(&ng_word.word)
                || self.author_name().contains(&ng_word.word)
        })
    }
}

// for thread
impl<T: ResState> NgWordRestrictable for (&Res<T>, String) {
    fn contains_ng_word(&self, ng_words: &[NgWord]) -> bool {
        let (res, thread_name) = self;
        ng_words.iter().any(|ng_word| {
            res.body().contains(&ng_word.word)
                || res.mail().contains(&ng_word.word)
                || res.author_name().contains(&ng_word.word)
                || thread_name.contains(&ng_word.word)
        })
    }
}
