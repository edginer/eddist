use std::fmt::Display;

use chrono::{DateTime, Utc};
use eddist_core::domain::client_info::ClientInfo;
use md5::{Digest, Md5};
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetadentType {
    None,
    Verbose,
    VVerbose,
    VVVerbose,
}

impl From<MetadentType> for Option<&str> {
    fn from(value: MetadentType) -> Self {
        match value {
            MetadentType::None => None,
            MetadentType::Verbose => Some("v"),
            MetadentType::VVerbose => Some("vv"),
            MetadentType::VVVerbose => Some("vvv"),
        }
    }
}

impl From<&str> for MetadentType {
    fn from(value: &str) -> Self {
        match value {
            "v" => MetadentType::Verbose,
            "vv" => MetadentType::VVerbose,
            "vvv" => MetadentType::VVVerbose,
            _ => MetadentType::None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Metadent {
    None,
    Enables(EnabledMetadent),
}

#[derive(Debug, Clone)]
pub struct EnabledMetadent {
    metadent_type: MetadentType,
    level: u32,
    ident_str: String,
}

impl Display for Metadent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Metadent::None => write!(f, ""),
            Metadent::Enables(EnabledMetadent {
                metadent_type,
                level,
                ident_str,
            }) => match metadent_type {
                MetadentType::None => write!(f, ""), // maybe unreachable but recoverable
                MetadentType::Verbose => write!(f, "</b>(L{})<b>", level),
                MetadentType::VVerbose => write!(f, "</b>({})<b>", ident_str),
                MetadentType::VVVerbose => write!(f, "</b>(L{} {})<b>", level, ident_str),
            },
        }
    }
}

impl Metadent {
    pub fn new(
        metadent_type: MetadentType,
        client_info: &ClientInfo,
        datetime: DateTime<Utc>,
    ) -> Self {
        if metadent_type == MetadentType::None {
            Metadent::None
        } else {
            let seed = generate_date_seed(datetime);
            let ident_str = generate_meta_ident(
                client_info.asn_num,
                &client_info.ip_addr,
                &client_info.user_agent,
                seed,
            );
            Metadent::Enables(EnabledMetadent {
                metadent_type,
                level: client_info.tinker.as_ref().map_or(0, |x| x.level()),
                ident_str,
            })
        }
    }
}

// for !metadent:vv, !metadent:vvv (vvv is currently disabled)
// (XXYY-zABB):
//   XX is generated from asn number ((asn + date_seed) % (len(a-zA-Z0-9))^2 to 2 byte char array to string)
//   YY is generated from ip_addr (if v6, only use first 4 segments)
//   z is 4 if v4, 6 if v6 (this segment does not use date_seed)
//   A is generated from type of Browser
//   BB is generated from UA
fn generate_meta_ident(asn: u32, ip_addr: &str, ua: &str, seed: u32) -> String {
    let alpha_char_62_to_ascii = |x: u8| match x {
        0..=9 => x + b'0',
        10..=35 => (x - 10) + b'A',
        36..=61 => (x - 36) + b'a',
        _ => b'0',
    };
    let num_to_2byte_chars = |x: u32| {
        let (first, second) = ((x / 62) as u8, (x % 62) as u8);
        vec![first, second]
            .into_iter()
            .map(alpha_char_62_to_ascii)
            .map(|x| x as char)
            .collect::<String>()
    };

    let (xx, _) = asn.overflowing_add(seed);
    let xx = xx % (62 * 62);
    let xx = num_to_2byte_chars(xx);

    let is_v6 = ip_addr.contains(':');
    let yy = ip_addr
        .split(if is_v6 { ':' } else { '.' })
        .take(4)
        .map(|x| {
            if is_v6 {
                if x.is_empty() {
                    0u64
                } else {
                    u64::from_str_radix(x, 16).unwrap_or(0)
                }
            } else {
                x.parse::<u64>().unwrap_or(0)
            }
        })
        .sum::<u64>()
        + seed as u64;

    let yy = (yy % (62 * 62)) as u32;
    let yy = num_to_2byte_chars(yy);
    let z = if is_v6 { 6 } else { 4 };

    let a = if ua.contains("Mate") {
        0
    } else if ua.contains("twinkle") {
        1
    } else if ua.contains("mae") {
        2
    } else if ua.contains("Siki") {
        3
    } else if ua.contains("Xeno") {
        4
    } else if ua.contains("ThreadMaster") {
        5
    } else {
        6
    } + seed;

    let a = (a % 62) as u8;
    let a = alpha_char_62_to_ascii(a) as char;

    let mut hasher = Md5::new();
    hasher.update(ua);
    let bb = hasher.finalize();

    let bb = bb
        .iter()
        .map(|x| *x as char)
        .filter(|x| x.is_ascii_alphanumeric())
        .take(2)
        .collect::<String>();

    format!("{xx}{yy}-{z}{a}{bb}")
}

fn generate_date_seed(time: DateTime<Utc>) -> u32 {
    let n = time.timestamp() as u64;
    let seed = (n / (60 * 60 * 24) / 7) % i32::MAX as u64;
    rand::rngs::StdRng::seed_from_u64(seed).gen()
}
