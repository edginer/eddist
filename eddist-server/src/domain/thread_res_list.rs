use super::{res_view::ResView, thread::Thread};

#[derive(Debug, Clone)]
pub struct ThreadResList {
    pub thread: Thread,
    pub res_list: Vec<ResView>,
}

impl ThreadResList {
    pub fn get_sjis_thread_res_list(&self, default_name: &str) -> Vec<u8> {
        let text = self
            .res_list
            .iter()
            .enumerate()
            .map(|(idx, x)| {
                x.get_sjis_bytes(
                    default_name,
                    if idx == 0 {
                        Some(&self.thread.title)
                    } else {
                        None
                    },
                )
                .get_inner()
            })
            .fold(Vec::new(), |mut cur, mut next| {
                cur.append(&mut next);
                cur
            });

        text
    }
}
