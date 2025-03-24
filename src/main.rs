use actix_web::{get, App, HttpRequest, HttpServer, Result};
use pages::storage::{backend_filesystem::PageStorageBackendFilesystem, PageStorageRead};

#[get("/page/{site_id}/{url:.*}")]
async fn asset(req: HttpRequest) -> Result<String> {
    let v1: String = req.match_info().get("site_id").unwrap().parse().unwrap();
    let v2: String = req.match_info().query("url").parse().unwrap();

    let storage = PageStorageBackendFilesystem::new(
        "C:/Users/anmei/Documents/MSS-LLC/pages/test_dir".to_string()
    ).unwrap();

    match storage.asset_contents(&v1, &v2) {
        Ok(content) => Ok(String::from_utf8(content).unwrap()),
        Err(_) => Ok("Error".to_string())
    }
}

#[get("/page/{site_id}")]
async fn index(req: HttpRequest) -> Result<String> {
    let v1: String = req.match_info().get("site_id").unwrap().parse().unwrap();
    let v2: String = req.match_info().query("url").parse().unwrap();

    let storage = PageStorageBackendFilesystem::new(
        "C:/Users/anmei/Documents/MSS-LLC/pages/test_dir".to_string()
    ).unwrap();

    let fmt = format!("{}/index.html", &v2);

    match storage.asset_contents(&v1, &fmt) {
        Ok(content) => Ok(String::from_utf8(content).unwrap()),
        Err(_) => Ok("Error".to_string())
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = setup_logger();

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(asset)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}