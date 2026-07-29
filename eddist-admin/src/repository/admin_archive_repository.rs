use core::str;

use aws_sdk_s3::{Client, primitives::ByteStream};
use eddist_core::domain::sjis_str::SJisStr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[async_trait::async_trait]
pub(crate) trait AdminArchiveRepository: Send + Sync {
    async fn get_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedThread>;
    async fn get_archived_admin_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedAdminThread>;
    async fn update_response(
        &self,
        board_key: &str,
        thread_number: u64,
        update_res_list: &[ArchivedResUpdate],
    ) -> anyhow::Result<()>;
    async fn delete_response(
        &self,
        board_key: &str,
        thread_number: u64,
        res_order: u64,
        keep_id: bool,
    ) -> anyhow::Result<()>;
    async fn delete_thread(&self, board_key: &str, thread_number: u64) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArchivedResUpdate {
    pub res_order: u64,
    pub author_name: String,
    pub email: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct AdminArchiveRepositoryImpl {
    client: Client,
    bucket: String,
}

impl AdminArchiveRepositoryImpl {
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ArchivedRes {
    pub name: String,
    pub mail: String,
    pub date: String,
    pub author_id: Option<String>,
    pub body: String,
    pub is_abone: bool,
    pub order: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ArchivedAdminRes {
    pub name: String,
    pub mail: String,
    pub date: String,
    pub author_id: Option<String>,
    pub ip_addr: String,
    pub authed_token_id: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ArchivedThread {
    pub title: String,
    pub responses: Vec<ArchivedRes>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ArchivedAdminThread {
    pub title: String,
    pub responses: Vec<ArchivedAdminRes>,
}

#[async_trait::async_trait]
impl AdminArchiveRepository for AdminArchiveRepositoryImpl {
    async fn get_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedThread> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(format!("{board_key}/dat/{thread_number}.dat"))
            .send()
            .await?;

        let dat_bytes = output.body.collect().await?.into_bytes().to_vec();

        let utf8_str = if let Ok(dat_bytes) = str::from_utf8(&dat_bytes) {
            dat_bytes.to_string()
        } else {
            encoding_rs::SHIFT_JIS.decode(&dat_bytes).0.to_string()
        };

        let a_thread = convert_dat_file_to_res(&utf8_str);

        Ok(a_thread)
    }

    async fn get_archived_admin_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedAdminThread> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(format!("{board_key}/admin/{thread_number}.dat"))
            .send()
            .await?;

        let dat_bytes = output.body.collect().await?.into_bytes().to_vec();

        let utf8_str = if let Ok(dat_bytes) = str::from_utf8(&dat_bytes) {
            dat_bytes.to_string()
        } else {
            encoding_rs::SHIFT_JIS.decode(&dat_bytes).0.to_string()
        };

        let a_thread = convert_admin_dat_file_to_res(&utf8_str);

        Ok(a_thread)
    }

    async fn update_response(
        &self,
        board_key: &str,
        thread_number: u64,
        update_res_list: &[ArchivedResUpdate],
    ) -> anyhow::Result<()> {
        let mut a_thread = self.get_thread(board_key, thread_number).await?;

        for update in update_res_list {
            let res = a_thread
                .responses
                .get_mut(update.res_order as usize)
                .ok_or(anyhow::anyhow!(
                    "Response order {} not found in thread {}",
                    update.res_order,
                    thread_number
                ))?;

            res.name = update.author_name.clone();
            res.mail = update.email.clone();
            res.body = update.body.clone();
        }

        let dat = convert_reses_to_dat_file(a_thread.responses, &a_thread.title);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(format!("{board_key}/dat/{thread_number}.dat"))
            .body(ByteStream::from(dat))
            .send()
            .await?;

        Ok(())
    }

    async fn delete_response(
        &self,
        board_key: &str,
        thread_number: u64,
        res_order: u64,
        keep_id: bool,
    ) -> anyhow::Result<()> {
        let mut a_thread = self.get_thread(board_key, thread_number).await?;
        let resp = a_thread
            .responses
            .get_mut(res_order as usize)
            .ok_or(anyhow::anyhow!(
                "Response order {} not found in thread {}",
                res_order,
                thread_number
            ))?;
        resp.is_abone = true;
        if !keep_id {
            resp.author_id = None;
        }

        let dat = convert_reses_to_dat_file(a_thread.responses, &a_thread.title);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(format!("{board_key}/dat/{thread_number}.dat"))
            .body(ByteStream::from(dat))
            .send()
            .await?;

        Ok(())
    }

    async fn delete_thread(&self, board_key: &str, thread_number: u64) -> anyhow::Result<()> {
        let src = format!("{board_key}/dat/{thread_number}.dat");
        let dst = format!("{src}.deleted");
        let get_result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&src)
            .send()
            .await;
        if let Ok(output) = get_result {
            let data = output.body.collect().await?.into_bytes();
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&dst)
                .body(ByteStream::from(data))
                .send()
                .await?;
            self.client
                .delete_object()
                .bucket(&self.bucket)
                .key(&src)
                .send()
                .await?;
        }

        Ok(())
    }
}

fn convert_dat_file_to_res(dat_file: &str) -> ArchivedThread {
    let thread_name_line = dat_file.lines().next().unwrap_or_default().to_string();
    let thread_name = thread_name_line
        .split("<>")
        .last()
        .unwrap_or_default()
        .to_string();

    let responses = dat_file
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let split: Vec<&str> = line.split("<>").collect();
            if split.len() < 4 {
                return None;
            }

            let date_and_author_id_split: Vec<&str> = split[2].split(" ID:").collect();
            let date = date_and_author_id_split[0];
            let author_id = date_and_author_id_split.get(1).map(|s| s.to_string());
            let body = split[3];

            // A real post's name/mail/body are free-form user (or, via the edit API,
            // admin) text, so a poster could deliberately write name="あぼーん",
            // body="あぼーん" to spoof an abone line. To tell a genuine abone apart:
            // - id-stripped abone never has an " ID:" segment, which a real post
            //   always has (the renderer always appends the real author_id), so
            //   ID-absence alone is an unspoofable, server-controlled signal.
            // - id-kept abone does carry a real ID, so that signal is unavailable.
            //   Instead it replaces the date sub-field with a fixed "あぼーん"
            //   sentinel. Unlike name/mail/body, the date is never derived from any
            //   user- or admin-editable input in any rendering path (eddist-core's
            //   renderer, used by eddist-cron to write the original archived dat,
            //   uses the identical sentinel — see get_sjis_bytes), so it can never
            //   collide with a genuine post's real timestamp.
            let is_abone = if author_id.is_none() {
                split[0] == "あぼーん" && body.trim() == "あぼーん"
            } else {
                date == "あぼーん"
            };

            Some(ArchivedRes {
                name: split[0].to_string(),
                mail: split[1].to_string(),
                date: date.to_string(),
                author_id,
                body: if is_abone {
                    "あぼーん".to_string()
                } else {
                    body.to_string()
                },
                is_abone,
                order: idx as u64,
            })
        })
        .collect();

    ArchivedThread {
        title: thread_name,
        responses,
    }
}

fn convert_admin_dat_file_to_res(dat_file: &str) -> ArchivedAdminThread {
    let thread_name = dat_file
        .lines()
        .next()
        .unwrap_or_default()
        .split("<>")
        .last()
        .unwrap_or_default()
        .to_string();

    let responses = dat_file
        .lines()
        .filter_map(|line| {
            let split = line.split("<>").collect::<Vec<_>>();
            if split.len() < 6 {
                return None;
            }

            let date_and_author_id_split = split[2].split(" ID:").collect::<Vec<_>>();
            Some(ArchivedAdminRes {
                name: split[0].to_string(),
                mail: split[1].to_string(),
                date: date_and_author_id_split[0].to_string(),
                author_id: date_and_author_id_split.get(1).map(|s| s.to_string()),
                ip_addr: split[3].to_string(),
                authed_token_id: split[4].to_string(),
                body: split[5].to_string(),
            })
        })
        .collect();

    ArchivedAdminThread {
        title: thread_name,
        responses,
    }
}

fn convert_reses_to_dat_file(reses: Vec<ArchivedRes>, thread_title: &str) -> Vec<u8> {
    let sjis_array = reses
        .into_iter()
        .enumerate()
        .map(|(idx, res)| {
            if res.is_abone {
                let title = if idx == 0 {
                    thread_title.to_string()
                } else {
                    "".to_string()
                };
                match res.author_id.as_deref() {
                    // The date sub-field is replaced with a fixed "あぼーん" sentinel
                    // (matching eddist-core's get_sjis_bytes, which eddist-cron uses to
                    // write the original archived dat) instead of the real timestamp.
                    // Unlike name/mail/body, date is never derived from any user- or
                    // admin-editable input, so it can never collide with a genuine
                    // post's real timestamp — see convert_dat_file_to_res for the
                    // matching detection logic.
                    Some(author_id) => SJisStr::from(&format!(
                        "あぼーん<>あぼーん<>あぼーん ID:{}<> あぼーん <>{}\n",
                        author_id, title
                    ) as &str),
                    None => SJisStr::from(
                        &format!("あぼーん<>あぼーん<><> あぼーん<>{}\n", title) as &str
                    ),
                }
            } else {
                SJisStr::from(&format!(
                    "{}<>{}<>{} ID:{}<>{}<>{}\n",
                    res.name,
                    res.mail,
                    res.date,
                    res.author_id.as_deref().unwrap_or_default(),
                    res.body,
                    if idx == 0 {
                        thread_title.to_string()
                    } else {
                        "".to_string()
                    }
                ) as &str)
            }
        })
        .fold(Vec::new(), |mut cur, next| {
            cur.append(&mut next.get_inner());
            cur
        });

    encoding_rs::SHIFT_JIS
        .decode(&sjis_array)
        .0
        .into_owned()
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reses() -> Vec<ArchivedRes> {
        vec![
            ArchivedRes {
                name: "名無しさん".to_string(),
                mail: "".to_string(),
                date: "2024/01/01(月) 00:00:00.00".to_string(),
                author_id: Some("ABC123456".to_string()),
                body: "本文1".to_string(),
                is_abone: false,
                order: 0,
            },
            ArchivedRes {
                name: "荒らし".to_string(),
                mail: "".to_string(),
                date: "2024/01/01(月) 00:01:00.00".to_string(),
                author_id: Some("XYZ789012".to_string()),
                body: "削除対象".to_string(),
                is_abone: true,
                order: 1,
            },
        ]
    }

    #[test]
    fn test_abone_strip_id_round_trip() {
        let mut reses = sample_reses();
        reses[1].author_id = None; // strip id (current default behavior)

        let dat = convert_reses_to_dat_file(reses, "テストスレッド");
        let dat_str = String::from_utf8(dat).unwrap();

        let parsed = convert_dat_file_to_res(&dat_str);
        let abone_res = &parsed.responses[1];

        assert!(abone_res.is_abone);
        assert_eq!(abone_res.author_id, None);
    }

    #[test]
    fn test_abone_keep_id_round_trip() {
        let reses = sample_reses(); // second response is abone with author_id kept

        let dat = convert_reses_to_dat_file(reses, "テストスレッド");
        let dat_str = String::from_utf8(dat).unwrap();

        let parsed = convert_dat_file_to_res(&dat_str);
        let abone_res = &parsed.responses[1];

        assert!(abone_res.is_abone);
        assert_eq!(abone_res.author_id.as_deref(), Some("XYZ789012"));
        assert!(!abone_res.body.contains("削除対象"));
        assert!(!abone_res.name.contains("荒らし"));
    }

    #[test]
    fn test_genuine_post_spoofing_abone_text_is_not_misdetected() {
        // A real (non-deleted) post whose author deliberately set name="あぼーん"
        // and body="あぼーん" to mimic an abone line must NOT be misdetected as
        // abone: the real timestamp in the date sub-field is never producible via
        // any user- or admin-editable input, so it disambiguates this from a
        // genuine id-kept abone line (whose date sub-field is the "あぼーん"
        // sentinel, not a real timestamp).
        let spoofing_res = ArchivedRes {
            name: "あぼーん".to_string(),
            mail: "あぼーん".to_string(),
            date: "2024/01/01(月) 00:00:00.00".to_string(),
            author_id: Some("SPOOF12345".to_string()),
            body: "あぼーん".to_string(),
            is_abone: false,
            order: 0,
        };

        let dat = convert_reses_to_dat_file(vec![spoofing_res], "テストスレッド");
        let dat_str = String::from_utf8(dat).unwrap();

        let parsed = convert_dat_file_to_res(&dat_str);
        let res = &parsed.responses[0];

        assert!(!res.is_abone);
        assert_eq!(res.author_id.as_deref(), Some("SPOOF12345"));
        assert_eq!(res.body, "あぼーん");
    }
}
