use std::convert::Infallible;
use std::net::ToSocketAddrs;
use std::{env, io, path::PathBuf};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::time::{Duration,SystemTime, UNIX_EPOCH};
use headers::{authorization::Basic, Authorization, HeaderMapExt};
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use structopt::StructOpt;
use tracing::{debug, error, info};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use config::{Config, ConfigError, File as CFile};
//use webdav_handler::{body::Body, memls::MemLs, fakels::FakeLs, DavConfig, DavHandler};
use dav_server::{body::Body, memls::MemLs,DavConfig, DavHandler};
use vfs::WebdavDriveFileSystem;
use model::{Credentials, LoginRequest, LoginResponse, EncrypResponse};

mod vfs;
mod model;
mod cache;


#[derive(StructOpt, Debug)]
#[structopt(name = "webdav")]
struct Opt {
    /// Listen host
    #[structopt(long, env = "HOST", default_value = "0.0.0.0")]
    host: String,
    /// Listen port
    #[structopt(short, env = "PORT", long, default_value = "9867")]
    port: u16,
    /// WebDAV authentication username
    #[structopt(short = "U", long, env = "WEBDAV_AUTH_USER")]
    auth_user: Option<String>,
    /// WebDAV authentication password
    #[structopt(short = "W", long, env = "WEBDAV_AUTH_PASSWORD")]
    auth_password: Option<String>,

   
    #[structopt(short = "S", long, default_value = "10485760")]
    read_buffer_size: usize,

    #[structopt(long, default_value = "16777216")]
    upload_buffer_size: usize,

    /// Directory entries cache size
    #[structopt(long, default_value = "1000")]
    cache_size: u64,
    /// Directory entries cache expiration time in seconds
    #[structopt(long, default_value = "600")]
    cache_ttl: u64,
    /// Root directory path
    #[structopt(long, default_value = "/")]
    root: String,
    /// Working directory, refresh_token will be stored in there if specified
    #[structopt(short = "w", long)]
    workdir: Option<PathBuf>,
        
    /// Prefix to be stripped off when handling request.
    #[structopt(long, env = "WEBDAV_STRIP_PREFIX")]
    strip_prefix: Option<String>,

}

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) xiaolongyunpan/3.2.7 Chrome/100.0.4896.143 Electron/18.2.0 Safari/537.36";


#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "native-tls-vendored")]
    openssl_probe::init_ssl_cert_env_vars();

    let opt = Opt::from_args();
    let auth_user = opt.auth_user;
    let auth_pwd = opt.auth_password;

    tracing_subscriber::fmt::init();
    if env::var("RUST_LOG").is_err() {
       env::set_var("RUST_LOG", "fast_webdav=info,reqwest=warn");
    }

    if (auth_user.is_some() && auth_pwd.is_none()) || (auth_user.is_none() && auth_pwd.is_some()) {
        anyhow::bail!("auth-user and auth-password should be specified together.");
    }


    // 读取配置文件
    // let mut settings = Config::default();
    // let config_path = Path::new("config.yaml");
    // settings.set_default("token", opt.stariver_token).unwrap();
    // let token:String = settings.get("token").unwrap();
   
    
    let fs = WebdavDriveFileSystem::new(opt.root, opt.cache_size, opt.cache_ttl,opt.upload_buffer_size,false,false)
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "initialize WebdavDriveFileSystem file system failed",
            )
        })?;
    info!("WebdavDriveFileSystem file system initialized");
    let mut dav_server_builder = DavHandler::builder()
        .filesystem(Box::new(fs))
        .locksystem(MemLs::new())
        .read_buf_size(opt.read_buffer_size)
        .autoindex(true)
        .redirect(true);
    if let Some(prefix) = opt.strip_prefix {
        dav_server_builder = dav_server_builder.strip_prefix(prefix);
    }

    let dav_server = dav_server_builder.build_handler();

    debug!(
        read_buffer_size = opt.read_buffer_size,
        auto_index = true,
        "webdav handler initialized"
    );

    let addr = (opt.host, opt.port)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    info!("listening on {:?}", addr);

    let make_service = hyper::service::make_service_fn(move |_| {
        let auth_user = auth_user.clone();
        let auth_pwd = auth_pwd.clone();
        let should_auth = auth_user.is_some() && auth_pwd.is_some();
        let dav_server = dav_server.clone();
        async move {
            let func = move |req: hyper::Request<hyper::Body>| {
                let dav_server = dav_server.clone();
                let auth_user = auth_user.clone();
                let auth_pwd = auth_pwd.clone();
                async move {
                    if should_auth {
                        let auth_user = auth_user.unwrap();
                        let auth_pwd = auth_pwd.unwrap();
                        let user = match req.headers().typed_get::<Authorization<Basic>>() {
                            Some(Authorization(basic))
                                if basic.username() == auth_user
                                    && basic.password() == auth_pwd =>
                            {
                                basic.username().to_string()
                            }
                            Some(_) | None => {
                                // return a 401 reply.
                                let response = hyper::Response::builder()
                                    .status(401)
                                    .header(
                                        "WWW-Authenticate",
                                        "Basic realm=\"webdav\"",
                                    )
                                    .body(Body::from("Authentication required".to_string()))
                                    .unwrap();
                                return Ok(response);
                            }
                        };
                        let config = DavConfig::new().principal(user);
                        Ok::<_, Infallible>(dav_server.handle_with(config, req).await)
                    } else {
                        Ok::<_, Infallible>(dav_server.handle(req).await)
                    }
                }
            };
            Ok::<_, Infallible>(hyper::service::service_fn(func))
        }
    });

    let _ = hyper::Server::bind(&addr)
        .serve(make_service)
        .await
        .map_err(|e| error!("server error: {}", e));
    Ok(())
}

 async fn post_request<T, U>(client:&Client,url: String, req: &T) -> anyhow::Result<Option<U>>
    where
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        let url = reqwest::Url::parse(&url)?;
        let res = client
            .post(url.clone())
            .json(&req)
            .send()
            .await?
            .error_for_status();
        match res {
            Ok(res) => {
                if res.status() == StatusCode::NO_CONTENT {
                    return Ok(None);
                }
                // let res = res.json::<U>().await?;
                // Ok(Some(res))
                let res = res.text().await?;
                //println!("{}: {}", url, res);
                let res = serde_json::from_str(&res)?;
                // let res_obj = res.json::<U>().await?;
                Ok(Some(res))
            }
            Err(err) => {
                Err(err.into())
            }
        }
}