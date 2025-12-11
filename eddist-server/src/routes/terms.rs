use std::env;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::{app::AppState, repositories::terms_repository::TermsRepository};

/// Public API response for terms (excludes internal fields like id)
#[derive(Debug, Serialize)]
pub struct TermsResponse {
    pub content: String,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<eddist_core::domain::terms::Terms> for TermsResponse {
    fn from(terms: eddist_core::domain::terms::Terms) -> Self {
        TermsResponse {
            content: terms.content,
            updated_at: terms.updated_at,
        }
    }
}

pub async fn get_terms(State(state): State<AppState>) -> impl IntoResponse {
    // FIXME: Temporarily return hardcoded terms
    let domain = env::var("DOMAIN").unwrap_or("example.com".to_string());
    let contact_point = env::var("CONTACT_POINT").unwrap_or("abuse@example.com".to_string());

    let terms_content = serde_json::json!({
        "sections": [
            {
                "title": "第1条（適用範囲）",
                "content": "本利用規約（以下「本規約」といいます）は、当掲示板（以下「本サービス」といいます）を利用するすべてのユーザー（以下「利用者」といいます）に適用されます。利用者は、本サービスを利用することにより、本規約に同意したものとみなされます。"
            },
            {
                "title": "第2条（収集する情報）",
                "content": "本サービスは、利用者のIPアドレス、Cookie、その他端末を特定するための情報を収集し、以下の目的で使用します。",
                "list": [
                    "本サービスの運営及び管理",
                    "不正利用の防止及びセキュリティの向上",
                    "サービスの改善及び提供内容の最適化"
                ],
                "additional": "これらの情報は、本サービス運営のためにのみ利用され、以下の場合に加えて法執行機関等からの正当な要求に応じる場合、または利用者が同意した場合を除き、第三者に提供することはありません。",
                "additional_list": [
                    "書き込み時、また書き込み前の認証時に利用者の正当性を確認するために、いくつかのサービス(*1)に問い合わせる場合"
                ]
            },
            {
                "title": "第3条（書き込みの責任）",
                "content": "本サービスにおけるすべての書き込み（テキスト、画像、その他の情報を含む）は、その書き込みを行った利用者に全責任が属します。利用者は、以下に定める違法な書き込みや不適切な内容を投稿しないことに同意するものとします。"
            },
            {
                "title": "第4条（禁止事項）",
                "content": "利用者は、以下の行為を行ってはなりません。",
                "sections": [
                    {
                        "subtitle": "違法な書き込み",
                        "items": [
                            "名誉毀損、中傷、侮辱、脅迫など、他者の権利や名誉を侵害する内容",
                            "著作権、商標権、特許権、プライバシー権、肖像権などの知的財産権を侵害する内容",
                            "無断で個人情報（氏名、住所、電話番号、メールアドレスなど）を公開する行為",
                            "法律で禁止されている行為を助長する内容や、犯罪行為に関与する内容"
                        ]
                    },
                    {
                        "subtitle": "その他不適切な書き込み",
                        "items": [
                            "過度に暴力的な表現、残虐な表現、児童ポルノを含む内容",
                            "虚偽の情報を流布し、混乱や誤解を招く内容",
                            "スパム、商業目的の宣伝、不正アクセス行為に関与する内容",
                            "スクリプトやbotなどを用いた自動書き込み行為",
                            "人種、民族、国籍、性別、宗教、障害、性的指向などに対する差別的な発言"
                        ]
                    }
                ]
            },
            {
                "title": "第5条（著作権）",
                "content": "利用者が本サービスに投稿した書き込みの著作権は、書き込みを行った利用者自身に属します。ただし、利用者は本サービス及び本サービスの関連サービス(*2)で投稿内容を使用、複製、編集、公開することについて、運営者に対して無期限かつ無償で非独占的な使用権を付与し、著作者人格権を行使しないことに同意します。利用者は、利用者自身の書き込みが第三者によって無断で転載されることを防止するため、本サービスに書き込みを行う際には原則、本サービスならびに本サービスに関連するサービス以外への転載を許諾しないものとして書き込むことに同意します。"
            },
            {
                "title": "第6条（違反行為への対応）",
                "content": "本サービスの運営側は、利用者の書き込み内容が本規約に違反している、または不適切であると判断した場合、当該書き込みを事前通知なく削除する権利を有します。また、法執行機関や、名誉毀損や中傷に関する被害者からの正当な求めがあった場合、投稿内容の削除および発信者情報の開示に応じることがあります。また、違反行為を繰り返す利用者に対してはアカウントの一時停止などの措置を取ることがあります。"
            },
            {
                "title": "第7条（免責事項）",
                "content": "本サービスは、利用者が本サービスの利用に関連して被ったあらゆる損害等について、一切の責任を負いません。利用者は、自己の責任で本サービスを利用するものとし、運営側に対して一切の賠償請求を行わないものとします。"
            },
            {
                "title": "第8条（規約の改定）",
                "content": "本規約は、必要に応じて改定されることがあります。改定後の規約は、本サービス上に掲載された時点で効力を発生します。利用者は、定期的に本規約を確認する義務を負い、改定後も本サービスの利用を継続した場合、改定内容に同意したものとみなされます。"
            }
        ],
        "footnotes": [
            "例: hCaptcha, Cloudflare Turnstile, Spur",
            format!("本サービスの運営者、もしくは運営者が委託する第三者が運営するサービス、加えていずれの場合も本サービスが使用するドメイン({})を含むサービス", domain)
        ],
        "contact": contact_point
    });

    let mut resp = Json(terms_content).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
}
