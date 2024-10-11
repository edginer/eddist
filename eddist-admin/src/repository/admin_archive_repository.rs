use core::str;

use eddist_core::domain::sjis_str::SJisStr;
use s3::Bucket;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub trait AdminArchiveRepository: Send + Sync + Clone {
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
pub struct AdminArchiveRepositoryImpl(Bucket);

impl AdminArchiveRepositoryImpl {
    pub fn new(bucket: Bucket) -> Self {
        Self(bucket)
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

impl AdminArchiveRepository for AdminArchiveRepositoryImpl {
    async fn get_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedThread> {
        let dat_file = self
            .0
            .get_object(format!("{board_key}/dat/{thread_number}.dat"))
            .await?;

        let dat_bytes = dat_file.to_vec();

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
        let dat_file = self
            .0
            .get_object(format!("{board_key}/admin/{thread_number}.dat"))
            .await?;

        let dat_bytes = dat_file.to_vec();

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

        self.0
            .put_object(format!("{board_key}/dat/{thread_number}.dat"), &dat)
            .await?;

        Ok(())
    }

    async fn delete_response(
        &self,
        board_key: &str,
        thread_number: u64,
        res_order: u64,
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

        let dat = convert_reses_to_dat_file(a_thread.responses, &a_thread.title);

        self.0
            .put_object(format!("{board_key}/dat/{thread_number}.dat"), &dat)
            .await?;

        Ok(())
    }

    async fn delete_thread(&self, board_key: &str, thread_number: u64) -> anyhow::Result<()> {
        self.0
            .delete_object(format!("{board_key}/dat/{thread_number}.dat"))
            .await?;

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
            Some(ArchivedRes {
                name: split[0].to_string(),
                mail: split[1].to_string(),
                date: date_and_author_id_split[0].to_string(),
                author_id: date_and_author_id_split.get(1).map(|s| s.to_string()),
                body: split[3].to_string(),
                is_abone: date_and_author_id_split.get(1).is_none()
                    && split[0] == "あぼーん"
                    && split[3].trim() == "あぼーん",
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
                SJisStr::from(&format!(
                    "あぼーん<>あぼーん<><> あぼーん<>{}\n",
                    if idx == 0 {
                        thread_title.to_string()
                    } else {
                        "".to_string()
                    }
                ) as &str)
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
