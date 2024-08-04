#[tokio::main]
async fn main() {
    // Jobs:
    // - inactivate and archive (not to show thread list),
    // - archive (move to archive table)
    // - convert (to dat text file compressed by gzip and delete responses, and publish to Cloudflare R2)

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("Usage: eddist-cron <job> [args...]");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "inactivate" => {
            // inactivate and archive
            // - inactivate (set active to false)
        }
        "archive" => {
            // archive
            // - archive (move to archive table)
        }
        "convert" => {
            // convert
            // - convert (to dat text file compressed by gzip and delete responses, and publish to Cloudflare R2)
            todo!();
        }
        job => {
            eprintln!("Unknown job: {job}");
            std::process::exit(1);
        }
    }
}
