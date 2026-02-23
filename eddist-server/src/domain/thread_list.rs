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
        use std::fmt::Write;
        let mut text = String::with_capacity(self.thread_list.len() * 100);
        for thread in &self.thread_list {
            let _ = writeln!(
                text,
                "{}.dat<>{} ({})",
                thread.thread_number, thread.title, thread.response_count
            );
        }
        SHIFT_JIS.encode(&text).0.to_vec()
    }
}

#[derive(Debug, Clone)]
pub struct ThreadListWithMetadent {
    pub board: Board,
    pub thread_list: Vec<(Thread, String)>,
}

impl ThreadListWithMetadent {
    pub fn get_sjis_thread_list(&self) -> Vec<u8> {
        use std::fmt::Write;
        let mut text = String::with_capacity(self.thread_list.len() * 120);
        for (thread, metadent) in &self.thread_list {
            let _ = writeln!(
                text,
                "{}.dat<>{} [{}★] ({})",
                thread.thread_number, thread.title, metadent, thread.response_count
            );
        }
        SHIFT_JIS.encode(&text).0.to_vec()
    }
}
