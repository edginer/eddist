use super::{res_view::ResView, thread::Thread};

#[derive(Debug, Clone)]
pub struct ThreadResList {
    pub thread: Thread,
    pub res_list: Vec<ResView>,
}

impl ThreadResList {
    pub fn get_sjis_thread_res_list(&self, default_name: &str) -> Vec<u8> {
        let text = self
            .get_sjis_list_inner(default_name)
            .fold(Vec::new(), |mut cur, mut next| {
                cur.append(&mut next);
                cur
            });

        text
    }

    pub fn get_sjis_list_thread_res_list(&self, default_name: &str) -> Vec<Vec<u8>> {
        self.get_sjis_list_inner(default_name).collect()
    }

    fn get_sjis_list_inner<'a>(
        &'a self,
        default_name: &'a str,
    ) -> impl Iterator<Item = Vec<u8>> + 'a {
        self.res_list.iter().enumerate().map(move |(idx, x)| {
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
    }
}
