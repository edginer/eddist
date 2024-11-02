use eddist_core::domain::board::Board;
use encoding_rs::SHIFT_JIS;

use super::thread::Thread;

#[derive(Debug, Clone)]
pub struct ThreadList {
    pub board: Board,
    pub thread_list: Vec<Thread>,
}

impl ThreadList {
    pub fn get_sjis_thread_list(&self) -> Vec<u8> {
        let text = self
            .thread_list
            .iter()
            .map(|x| {
                format!(
                    "{}.dat<>{} ({})\n",
                    x.thread_number, x.title, x.response_count
                )
            })
            .fold(String::new(), |mut cur, next| {
                cur.push_str(&next);
                cur
            });

        SHIFT_JIS.encode(&text).0.to_vec()
    }
}
