use std::str;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter,Write};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration,SystemTime, UNIX_EPOCH};
use reqwest::multipart::{Form, Part};
use tracing_subscriber::fmt::format;
use url::Url;
use base64::{encode, decode};
use md5::{Md5, Digest as MDigest};
use sha1::{Sha1, Digest};
use anyhow::{Result, Context, Error};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use futures_util::future::{ready, ok, FutureExt};
use tracing::{debug, error, trace,info};
use dav_server::{
    davpath::DavPath,
    fs::{
        DavDirEntry, DavFile, DavFileSystem, DavMetaData, FsError, FsFuture, FsStream, OpenOptions,
        ReadDirMeta,DavProp
    },
};
use moka::future::{Cache as AuthCache};
use crate::cache::Cache;
use reqwest::{
    header::{HeaderMap, HeaderName,HeaderValue},
    StatusCode,
};
use tokio::{
    sync::{oneshot, RwLock},
    time,
};
use tokio::time::{sleep, Duration as TDuration};
use serde::de::DeserializeOwned;
use serde::{Serialize,Deserialize};
use quick_xml::de::from_str;
use quick_xml::{Writer, se};
use quick_xml::se::Serializer as XmlSerializer;
use serde_json::json;
use reqwest::header::RANGE;

pub use crate::model::*;


const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) xiaolongyunpan/3.2.7 Chrome/100.0.4896.143 Electron/18.2.0 Safari/537.36";
const API_URL:&str = "http://127.0.0.1:8000/";




#[derive(Clone)]
pub struct WebdavDriveFileSystem {
    auth_cache:AuthCache<String, String>,
    dir_cache: Cache,
    uploading: Arc<DashMap<String, Vec<WebdavFile>>>,
    root: PathBuf,
    client:reqwest::Client,
    upload_buffer_size: usize,
    skip_upload_same_size: bool,
    prefer_http_download: bool,
}
impl WebdavDriveFileSystem {
    pub async fn new(
        root: String,
        cache_size: u64,
        cache_ttl: u64,
        upload_buffer_size: usize,
        skip_upload_same_size: bool,
        prefer_http_download: bool,
    ) -> Result<Self> {
        let dir_cache = Cache::new(cache_size, cache_ttl);
        debug!("dir cache initialized");
        let root = if root.starts_with('/') {
            PathBuf::from(root)
        } else {
            Path::new("/").join(root)
        };

        let client = reqwest::Client::builder()
            .pool_idle_timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(300))
            .timeout(Duration::from_secs(300))
            .build()?;


        let auth_cache = AuthCache::new(2);

        let driver: WebdavDriveFileSystem = Self {
            auth_cache,
            dir_cache,
            uploading: Arc::new(DashMap::new()),
            root,
            client,
            upload_buffer_size,
            skip_upload_same_size,
            prefer_http_download,
        };

        driver.dir_cache.invalidate_all();
        Ok(driver)
    }

 
   
    async fn get_request<T, U>(&self, url: String, req: &T) -> Result<Option<U>>
    where
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert("accept", "application/json, text/plain, */*".parse().unwrap());
        headers.insert("sec-fetch-dest", "empty".parse().unwrap());
        headers.insert("client-platform", "mac".parse().unwrap());
        headers.insert("accept-language", "zh".parse().unwrap());
        headers.insert("client-version", "3.2.7".parse().unwrap());
        headers.insert("user-agent", UA.parse().unwrap());
        headers.insert("content-type", "application/json;charset=UTF-8".parse().unwrap());
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        headers.insert("sec-fetch-mode", "cors".parse().unwrap());
        let res = self
            .client
            .get(url.clone())
            .headers(headers)
            .json(&req)
            .send()
            .await?
            .error_for_status();
        match res {
            Ok(res) => {
                if res.status() == StatusCode::NO_CONTENT {
                    return Ok(None);
                }
                //let res = res.json::<U>().await?;
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


    async fn post_request<T, U>(&self, url: String, req: &T) -> Result<Option<U>>
    where
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert("accept", "application/json, text/plain, */*".parse().unwrap());
        headers.insert("sec-fetch-dest", "empty".parse().unwrap());
        headers.insert("client-platform", "mac".parse().unwrap());
        headers.insert("accept-language", "zh".parse().unwrap());
        headers.insert("client-version", "3.2.7".parse().unwrap());
        headers.insert("user-agent", UA.parse().unwrap());
        headers.insert("content-type", "application/json;charset=UTF-8".parse().unwrap());
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        headers.insert("sec-fetch-mode", "cors".parse().unwrap());
        let res = self
            .client
            .post(url.clone())
            .headers(headers)
            .json(&req)
            .send()
            .await?
            .error_for_status();
        match res {
            Ok(res) => {
                if res.status() == StatusCode::NO_CONTENT {
                    return Ok(None);
                }
                //let res = res.json::<U>().await?;
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


    async fn post_body_request<U>(&self, url: String, form:Form) -> Result<Option<U>>
    where
        U: DeserializeOwned,
    {
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert("accept", "application/json, text/plain, */*".parse().unwrap());
        headers.insert("sec-fetch-dest", "empty".parse().unwrap());
        headers.insert("client-platform", "mac".parse().unwrap());
        headers.insert("accept-language", "zh".parse().unwrap());
        headers.insert("client-version", "3.2.7".parse().unwrap());
        headers.insert("user-agent", UA.parse().unwrap());
        headers.insert("content-type", "application/json;charset=UTF-8".parse().unwrap());
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        headers.insert("sec-fetch-mode", "cors".parse().unwrap());
        let res = self
            .client
            .post(url.clone())
            .headers(headers)
            .multipart(form)
            .send()
            .await?
            .error_for_status();
        match res {
            Ok(res) => {
                if res.status() == StatusCode::NO_CONTENT {
                    return Ok(None);
                }
                //let res = res.json::<U>().await?;
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



    fn set_upload_buffer_size(&mut self, new_size: u64) {
        self.upload_buffer_size = new_size as usize;
    }

    async fn create_folder(&self,dav_path: &DavPath, parent_id:&str, folder_name: &str) -> Result<WebdavFile> {

        let path = self.normalize_dav_path(dav_path);
        let parent_path = path.parent().ok_or(FsError::NotFound)?;

        if parent_id=="0" && parent_path.to_path_buf().to_string_lossy()=="/"{
            error!("根目录的文件夹需要在provider.ini中指定");
            panic!("根目录的文件夹需要在provider.ini中指定")
        }

        let path_str = parent_path.to_string_lossy().into_owned();
        let parent_file = match self.get_by_path(&path_str).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("获取上级目录信息失败: {:?}", err);
                panic!("获取上级目录信息失败: {:?}", err)
            }
        };

        let create_req = CreateFolderRequest{
            name:folder_name,
            parent_id:parent_id,
            parend_file:parent_file.clone()
        };
        let create_url = format!("{}{}/create_folder",API_URL,parent_file.provider.unwrap()); 
        let created_folder:WebdavFile = match self.post_request(create_url, &create_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("创建文件夹失败: {:?}", err);
                panic!("创建文件夹失败: {:?}", err)
            }
        };

        Ok(created_folder)
    }

    pub async fn remove_file(&self,file: &WebdavFile) -> Result<()> {
        if file.id=="0"{
            error!("根目录的文件夹无法修改或删除");
            panic!("根目录的文件夹无法修改或删除")
        }

        let remove_req = RemoveFileRequest{
            file:file.clone()
        };
        let remove_url = format!("{}{}/remove_file",API_URL,file.clone().provider.unwrap()); 
        let removed_file:WebdavFile = match self.post_request(remove_url, &remove_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("删除文件失败: {:?}", err);
                panic!("删除文件失败: {:?}", err)
            }
        };
        Ok(())
    }

    pub async fn rename_file(&self, file: &WebdavFile, new_name: &str) -> Result<()> {
        let rename_req = RenameFileRequest{
            file:file.clone(),
            new_name:new_name,
        };
        let rename_url = format!("{}{}/rename_file",API_URL,file.clone().provider.unwrap()); 
        let renamed_file:WebdavFile = match self.post_request(rename_url, &rename_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("重命名文件失败: {:?}", err);
                panic!("重命名文件失败: {:?}", err)
            }
        };
        Ok(())
    }


    pub async fn move_file(&self, file: &WebdavFile, new_parent_id: &str) -> Result<()> {
        let move_req = MoveFileRequest{
            file:file.clone(),
            new_parent_id:new_parent_id,
        };
        let move_url = format!("{}{}/move_file",API_URL,file.clone().provider.unwrap()); 
        let moved_file:WebdavFile = match self.post_request(move_url, &move_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("重命名文件失败: {:?}", err);
                panic!("重命名文件失败: {:?}", err)
            }
        };
        Ok(())
    }

    pub async fn copy_file(&self, file: &WebdavFile, new_parent_id: &str) -> Result<()> {
        let copy_req = CopyFileRequest{
            file:file.clone(),
            new_parent_id:new_parent_id,
        };
        let copy_url = format!("{}{}/copy_file",API_URL,file.clone().provider.unwrap()); 
        let copyied_file:WebdavFile = match self.post_request(copy_url, &copy_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("重命名文件失败: {:?}", err);
                panic!("重命名文件失败: {:?}", err)
            }
        };
        Ok(())
    }

    pub async fn get_useage_quota(&self) -> Result<(u64, u64)> {
        Ok((1024, 1000000000000))
    }

    async fn list_files_and_cache( &self, path_str: String, parent_file_id: String)-> Result<Vec<WebdavFile>>{
        info!(path = %path_str, parent_id=%parent_file_id,"cache dir");
        let req:FilesListRequest=FilesListRequest {path_str:json!(path_str),parent_file_id:json!(parent_file_id)}; 
        let mut file_list:Vec<WebdavFile>=vec![];
        if parent_file_id == '0'.to_string() && path_str == '/'.to_string() {
            let list_url = API_URL.to_string();
            let files:Vec<WebdavFile> = match self.get_request(list_url, &req).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("文件列表请求失败: {:?}", err);
                    panic!("文件列表请求失败: {:?}", err)
                }
            };
            file_list.extend(files);
        }else {
            let parent_file = match self.get_by_path(&path_str).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("文件列表请求失败: {:?}", err);
                    panic!("文件列表请求失败: {:?}", err)
                }
            };
            let list_url = format!("{}{}/list",API_URL,parent_file.provider.unwrap());
            let files:Vec<WebdavFile> = match self.post_request(list_url, &req).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("文件列表请求失败: {:?}", err);
                    panic!("文件列表请求失败: {:?}", err)
                }
            };
            file_list.extend(files);
        }   
        
        self.cache_dir(path_str,file_list.clone()).await;
        Ok(file_list)

    }

    async fn cache_dir(&self, dir_path: String, files: Vec<WebdavFile>) {
        trace!(path = %dir_path, count = files.len(), "cache dir");
        self.dir_cache.insert(dir_path, files).await;
    }

    fn find_in_cache(&self, path: &Path) -> Result<Option<WebdavFile>, FsError> {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy().into_owned();
            let file_name = path
                .file_name()
                .ok_or(FsError::NotFound)?
                .to_string_lossy()
                .into_owned();
            let file = self.dir_cache.get(&parent_str).and_then(|files| {
                for file in &files {
                    if file.name == file_name {
                        return Some(file.clone());
                    }
                }
                None
            });
            Ok(file)
        } else {
            let root = WebdavFile::new_root();
            Ok(Some(root))
        }
    }


    fn find_file_in_cache(&self, parent_path: &Path,file_id:&str) -> Result<Option<WebdavFile>, FsError> {
        let parent_str = parent_path.to_string_lossy().into_owned();
        let file = self.dir_cache.get(&parent_str).and_then(|files| {
            for file in &files {
                if file.id == file_id {
                    return Some(file.clone());
                }
            }
            None
        });
        Ok(file)
    }

    async fn read_dir_and_cache(&self, path: PathBuf) -> Result<Vec<WebdavFile>, FsError> {
        let path_str = path.to_string_lossy().into_owned();
        debug!(path = %path_str, "read_dir and cache");
        let parent_file_id = if path_str == "/" {
            "0".to_string()
        } else {
            match self.find_in_cache(&path) {
                Ok(Some(file)) => file.id,
                _ => {
                    if let Ok(Some(file)) = self.get_by_path(&path_str).await {
                        file.id
                    } else {
                        return Err(FsError::NotFound);
                    }
                }
            }
        };
        let mut files = if let Some(files) = self.dir_cache.get(&path_str) {
            files
        } else {
            self.list_files_and_cache(path_str, parent_file_id.clone()).await.map_err(|_| FsError::NotFound)?
        };

        let uploading_files = self.list_uploading_files(&parent_file_id);
        if !uploading_files.is_empty() {
            debug!("added {} uploading files", uploading_files.len());
            files.extend(uploading_files);
        }

        Ok(files)
    }


    fn list_uploading_files(&self, parent_file_id: &str) -> Vec<WebdavFile> {
        self.uploading
            .get(parent_file_id)
            .map(|val_ref| val_ref.value().clone())
            .unwrap_or_default()
    }


    fn remove_uploading_file(&self, parent_file_id: &str, name: &str) {
        if let Some(mut files) = self.uploading.get_mut(parent_file_id) {
            if let Some(index) = files.iter().position(|x| x.name == name) {
                files.swap_remove(index);
            }
        }
    }

    pub async fn get_by_path(&self, path: &str) -> Result<Option<WebdavFile>> {
        debug!(path = %path, "get file by path");
        if path == "/" || path.is_empty() {
            return Ok(Some(WebdavFile::new_root()));
        }
        let tpath = PathBuf::from(path);
        let path_str = tpath.to_string_lossy().into_owned();
        let file = self.find_in_cache(&tpath)?;
        if let Some(file) = file {
            Ok(Some(file))
        } else {
            let parts: Vec<&str> = path_str.split('/').collect();
            let parts_len = parts.len();
            let filename = parts[parts_len - 1];
            let mut prefix = PathBuf::from("/");
            for part in &parts[0..parts_len - 1] {
                let parent = prefix.join(part);
                prefix = parent.clone();
                let files = self.dir_cache.get(&parent.to_string_lossy().into_owned()).unwrap();
                if let Some(file) = files.iter().find(|f| &f.name == filename) {
                    return Ok(Some(file.clone()));
                }
            }
            Ok(Some(WebdavFile::new_root()))
        }
    
    }


    async fn get_file(&self, path: PathBuf) -> Result<Option<WebdavFile>, FsError> {

        let path_str = path.to_string_lossy().into_owned();
        debug!(path = %path_str, "get_file");

        // let pos = path_str.rfind('/').unwrap();
        // let path_length = path_str.len()-pos;
        // let path_name: String = path_str.chars().skip(pos+1).take(path_length).collect();

        let parts: Vec<&str> = path_str.split('/').collect();
        let parts_len = parts.len();
        let path_name = parts[parts_len - 1];

        // 忽略 macOS 上的一些特殊文件
        if path_name == ".DS_Store" || path_name.starts_with("._") {
            return Err(FsError::NotFound);
        }

        let file = self.find_in_cache(&path)?;
        if let Some(file) = file {
            trace!(path = %path.display(), file_id = %file.id, "file found in cache");
            Ok(Some(file))
        } else {

            debug!(path = %path.display(), "file not found in cache");
            // trace!(path = %path.display(), "file not found in cache");
            // if let Ok(Some(file)) = self.get_by_path(&path_str).await {
            //     return Ok(Some(file));
            // }
            let parts: Vec<&str> = path_str.split('/').collect();
            let parts_len = parts.len();
            let filename = parts[parts_len - 1];
            let mut prefix = PathBuf::from("/");
            for part in &parts[0..parts_len - 1] {
                let parent = prefix.join(part);
                prefix = parent.clone();
                let files = self.read_dir_and_cache(parent).await?;
                if let Some(file) = files.iter().find(|f| f.name == filename) {
                    trace!(path = %path.display(), file_id = %file.id, "file found in cache");
                    return Ok(Some(file.clone()));
                }
            }
            Ok(None)
        }

    }

    async fn get_download_url(&self,parent_dir:&PathBuf,file_id: &str) -> Result<String> {
        debug!("get_download_url from request");
        //需要修改 第一次的时候download_url为None去请求，成功后缓存，不为None的话判断是否过期如果过期则请求不过期则从缓存读取
        //需要修改缓存的方法
        let davfile = self.find_file_in_cache(parent_dir, file_id).unwrap().unwrap();
        match davfile.clone().download_url {
            Some(u)=>{
                if !is_url_expired(&u) {
                    return Ok(u);
                }
            },
            None=>{
                debug!("下载地址为空，开始请求新下载地址");
            }
        }

        let download_url = format!("{}{}/url",API_URL,davfile.clone().provider.unwrap());
        let donwload_url:String = match self.post_request(download_url, &davfile).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("文件下载地址获取失败: {:?}", err);
                panic!("文件下载地址获取失败: {:?}", err)
            }
        };
        Ok(donwload_url)
    }


    pub async fn download(&self, url: &str, play_headers:Option<String>, start_pos: u64, size: usize) -> Result<Bytes> {
        let end_pos = start_pos + size as u64 - 1;
        debug!(url = %url, start = start_pos, end = end_pos, "download file");
        let range = format!("bytes={}-{}", start_pos, end_pos);

        let headers = match play_headers {
            Some(res)=>res,
            None=>"".to_string(),
        };
        if headers.is_empty(){
            let res = self.client
            .get(url)
            .header(RANGE, range)
            .timeout(Duration::from_secs(120))
            .send()
            .await?
            .error_for_status()?;
            Ok(res.bytes().await?)
        }else {
            let header_map: HashMap<String, String> = serde_json::from_str(&headers)?;
            let mut pheaders = HeaderMap::new();
            for (key, value) in header_map.iter() {
                pheaders.insert(
                    HeaderName::from_bytes(key.as_bytes()).unwrap(),
                    HeaderValue::from_bytes(value.as_bytes()).unwrap(),
                );
            }
            let res = self.client
            .get(url)
            .header(RANGE,range)
            .headers(pheaders)
            .timeout(Duration::from_secs(120))
            .send()
            .await?
            .error_for_status()?;
            Ok(res.bytes().await?)

        }

        
        
    }


    pub async fn create_file_with_proof(&mut self,provider:&str,name: &str, parent_file_id: &str, hash:&str, size: u64) ->  Result<UploadInitResponse> {
        debug!(size = size,"create_file_with_proof");
        let sizeStr=size.to_string(); 
        
        let init_file_req = UploadInitRequest{
            provider: provider.to_string(),
            name: hash.to_string(),
            parent_file_id: name.to_string(),
            sha1: hash.to_string(),
            size: size,
        };

        let init_upload_url = format!("{}{}/init",API_URL,provider);
        let file_upload_init_res:UploadInitResponse = match  self.post_request(init_upload_url,&init_file_req).await{
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("初始化文件上传请求失败: {:?}", err);
                panic!("初始化文件上传请求失败: {:?}", err)
            }
        };

       debug!("输出创建文件信息开始");
       debug!("{:?}",file_upload_init_res);
       debug!("输出创建文件信息结束");

       if file_upload_init_res.code != 200 as u64 {
           error!("{}",&file_upload_init_res.message);
           panic!("{}",&file_upload_init_res.message);
       }

       &self.set_upload_buffer_size(file_upload_init_res.data.chunkSize);
        
        Ok(file_upload_init_res)

    }


    pub async fn get_pre_upload_info(&self,oss_args:&OssArgs) -> Result<String> {
        Ok(oss_args.sha1.clone())
    }

    pub async fn upload_chunk(&self, file:&WebdavFile, oss_args:&OssArgs, upload_id:&str, current_chunk:u64,body: Bytes) -> Result<(SliceUploadResponse)> {
        debug!(file_name=%file.name,upload_id = upload_id,current_chunk=current_chunk, "upload_chunk");
        let upload_req = SliceUploadRequest{
            file:file.clone(),
            oss_args:oss_args.clone(),
            upload_id:upload_id.to_string(),
            current_chunk:current_chunk
        };
        let req_str = serde_json::to_string(&upload_req).unwrap();
        let json_base_str = encode(req_str);
        //let json: serde_json::Value = serde_json::from_str(&req_str)?;
        let formfiledata: Part = Part::bytes(body.to_vec()).file_name("slice");
        let form = reqwest::multipart::Form::new()
            .part("filedata",formfiledata)
            .text("slice_req", json_base_str);

        let uploader_url = format!("{}{}/upload_chunk",API_URL,file.clone().provider.unwrap());
        let slice_upload_res:SliceUploadResponse = match self.post_body_request(uploader_url,form).await {
            Ok(res)=>res.unwrap(),
            Err(err)=>{
                error!("文件分片上传失败: {:?}", err);
                panic!("文件分片上传失败: {:?}", err)
            }
        };

        if slice_upload_res.code!=200 as u64 {
            error!("文件分片上传失败: {}", slice_upload_res.message);
            panic!("文件分片上传失败: {}", slice_upload_res.message)
        }

        Ok(slice_upload_res)
    }


    pub async fn complete_upload(&self,file:&WebdavFile, upload_tags:String, oss_args:&OssArgs, upload_id:&str)-> Result<()> {
        let complete_upload_req: CompleteUploadRequest = CompleteUploadRequest{
            file:file.clone(),
            oss_args:oss_args.clone(),
            upload_tags:upload_tags,
            upload_id:upload_id.to_string(),
        };

        //由于fastapi模型转换问题只好用form把json字符串提交过去再解析了
        // let req_str = serde_json::to_string(&complete_upload_req).unwrap();
        // let json_base_str = encode(req_str);
        // //let json_complete: serde_json::Value = serde_json::from_str(&req_str)?;
        // let form = reqwest::multipart::Form::new()
        //     .text("complete_req", json_base_str);

        let complete_url = format!("{}{}/complete_upload",API_URL,file.clone().provider.unwrap());
        let complete_uplad_res:CompleteUploadResponse = match self.post_request(complete_url, &complete_upload_req).await {
            Ok(res) => res.unwrap(),
            Err(err)  => {
                error!("文件分片上传失败: {:?}", err);
                panic!("文件分片上传失败: {:?}", err)
            }
        };

        if complete_uplad_res.status != 200 as u64{
            error!("文件分片上传失败: {}", complete_uplad_res.data);
            panic!("文件分片上传失败: {}", complete_uplad_res.data);
        }

        Ok(())
    }


    pub fn hmac_authorization(&self, req:&reqwest::Request,time:&str,oss_args:&OssArgs)->String{
        "hello".to_string()
    }
   

    fn normalize_dav_path(&self, dav_path: &DavPath) -> PathBuf {
        let path = dav_path.as_pathbuf();
        if self.root.parent().is_none() || path.starts_with(&self.root) {
            return path;
        }
        let rel_path = dav_path.as_rel_ospath();
        if rel_path == Path::new("") {
            return self.root.clone();
        }
        self.root.join(rel_path)
    }
}

impl DavFileSystem for WebdavDriveFileSystem {
    fn open<'a>(
        &'a self,
        dav_path: &'a DavPath,
        options: OpenOptions,
    ) -> FsFuture<Box<dyn DavFile>> {
        let path = self.normalize_dav_path(dav_path);
        let mode = if options.write { "write" } else { "read" };
        debug!(path = %path.display(), mode = %mode, "fs: open");
        async move {
            if options.append {
                // Can't support open in write-append mode
                error!(path = %path.display(), "unsupported write-append mode");
                return Err(FsError::NotImplemented);
            }
            let parent_path = path.parent().ok_or(FsError::NotFound)?;
            let parent_file = self
                .get_file(parent_path.to_path_buf())
                .await?
                .ok_or(FsError::NotFound)?;
            let sha1 = options.checksum.and_then(|c| {
                if let Some((algo, hash)) = c.split_once(':') {
                    if algo.eq_ignore_ascii_case("sha1") {
                        Some(hash.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            let dav_file = if let Some(mut file) = self.get_file(path.clone()).await? {
                if options.write && options.create_new {
                    return Err(FsError::Exists);
                }
                FastDavFile::new(self.clone(), file, parent_file.id,parent_path.to_path_buf(),options.size.unwrap_or_default(),sha1.clone())
            } else if options.write && (options.create || options.create_new) {

               
                if parent_file.id=="0" && parent_path.clone().to_path_buf().to_string_lossy()=="/"{
                    error!("无法上传文件到根目录");
                    panic!("无法上传文件到根目录")
                }

                let size = options.size;
                let name = dav_path
                    .file_name()
                    .ok_or(FsError::GeneralFailure)?
                    .to_string();

                // 忽略 macOS 上的一些特殊文件
                if name == ".DS_Store" || name.starts_with("._") {
                    return Err(FsError::NotFound);
                }
                let now = SystemTime::now();

                let file_path = dav_path.as_url_string();
                let mut hasher = Sha1::default();
                hasher.update(file_path.as_bytes());
                let hash_code = hasher.finalize();
                let hash_str = format!("{:X}",&hash_code).to_lowercase();

                let file_hash = match sha1.clone() {
                    Some(str)=>str,
                    None=>hash_str
                };

                let parent_folder_id = parent_file.id.clone();
                let file = WebdavFile {
                    id: "0".to_string(),
                    provider:parent_file.provider,
                    kind:0,
                    name: name,
                    parent_id: parent_folder_id,
                    size: size.unwrap_or(0).to_string(),
                    create_time: chrono::offset::Utc::now(),
                    download_url:None,
                    sha1:Some(file_hash),
                    play_headers:None,
                };
                let mut uploading = self.uploading.entry(parent_file.id.clone()).or_default();
                uploading.push(file.clone());

                FastDavFile::new(self.clone(), file, parent_file.id,parent_path.to_path_buf(),size.unwrap_or(0),sha1)
            } else {
                println!("FsError::NotFound");
                return Err(FsError::NotFound);
            };
            Ok(Box::new(dav_file) as Box<dyn DavFile>)
        }
        .boxed()
    }

    fn read_dir<'a>(
        &'a  self,
        path: &'a DavPath,
        _meta: ReadDirMeta,
    ) -> FsFuture<FsStream<Box<dyn DavDirEntry>>> {
        let path = self.normalize_dav_path(path);
        debug!(path = %path.display(), "fs: read_dir");
        async move {
            let files = self.read_dir_and_cache(path.clone()).await?;
            let mut v: Vec<Box<dyn DavDirEntry>> = Vec::with_capacity(files.len());
            for file in files {
                v.push(Box::new(file));
            }
            let stream = futures_util::stream::iter(v);
            Ok(Box::pin(stream) as FsStream<Box<dyn DavDirEntry>>)
        }
        .boxed()
    }


    fn create_dir<'a>(&'a self, dav_path: &'a DavPath) -> FsFuture<()> {
        let path = self.normalize_dav_path(dav_path);
        async move {
            let parent_path = path.parent().ok_or(FsError::NotFound)?;
            let parent_file = self
                .get_file(parent_path.to_path_buf())
                .await?
                .ok_or(FsError::NotFound)?;
            
            if !(parent_file.kind==0) {
                return Err(FsError::Forbidden);
            }
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy().into_owned();
                self.create_folder(dav_path,&parent_file.id,&name).await;
                self.dir_cache.invalidate(parent_path).await;
                Ok(())
            } else {
                Err(FsError::Forbidden)
            }
        }
        .boxed()
    }


    fn remove_dir<'a>(&'a self, dav_path: &'a DavPath) -> FsFuture<()> {
        let path = self.normalize_dav_path(dav_path);
        debug!(path = %path.display(), "fs: remove_dir");
        async move {

            let parent_path = path.parent().unwrap();

            let path_str = parent_path.to_string_lossy().into_owned();
            let parent_file = match self.get_by_path(&path_str).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("获取上级目录信息失败: {:?}", err);
                    panic!("获取上级目录信息失败: {:?}", err)
                }
            };
            if parent_file.id=="0" && parent_path.to_path_buf().to_string_lossy()=="/"{
                error!("根目录的文件夹无法修改或删除");
                panic!("根目录的文件夹无法修改或删除")
            }


            let file = self
                .get_file(path.clone())
                .await?
                .ok_or(FsError::NotFound)?;

            if !(file.kind==0) {
                return Err(FsError::Forbidden);
            }

            self.remove_file(&file)
                .await
                .map_err(|err| {
                    error!(path = %path.display(), error = %err, "remove directory failed");
                    FsError::GeneralFailure
                })?;
            self.dir_cache.invalidate(&path).await;
            self.dir_cache.invalidate_parent(&path).await;
            Ok(())
        }
        .boxed()
    }


    fn remove_file<'a>(&'a self, dav_path: &'a DavPath) -> FsFuture<()> {
        let path = self.normalize_dav_path(dav_path);
        debug!(path = %path.display(), "fs: remove_file");
        async move {
            let file = self
                .get_file(path.clone())
                .await?
                .ok_or(FsError::NotFound)?;

            self.remove_file(&file)
                .await
                .map_err(|err| {
                    error!(path = %path.display(), error = %err, "remove file failed");
                    FsError::GeneralFailure
                })?;
            self.dir_cache.invalidate_parent(&path).await;
            Ok(())
        }
        .boxed()
    }


    fn rename<'a>(&'a self, from_dav: &'a DavPath, to_dav: &'a DavPath) -> FsFuture<()> {
        let from = self.normalize_dav_path(from_dav);
        let to = self.normalize_dav_path(to_dav);
        debug!(from = %from.display(), to = %to.display(), "fs: rename");
        async move {


            let parent_path = to.parent().unwrap();

            let path_str = parent_path.to_string_lossy().into_owned();
            let parent_file = match self.get_by_path(&path_str).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("获取上级目录信息失败: {:?}", err);
                    panic!("获取上级目录信息失败: {:?}", err)
                }
            };
            if parent_file.id=="0" && parent_path.to_path_buf().to_string_lossy()=="/"{
                error!("根目录的文件夹无法修改或删除");
                panic!("根目录的文件夹无法修改或删除")
            }


            let is_dir;
            if from.parent() == to.parent() {
                // rename
                if let Some(name) = to.file_name() {
                    let file = self
                        .get_file(from.clone())
                        .await?
                        .ok_or(FsError::NotFound)?;
                    is_dir = if file.kind == 0 {
                        true
                    } else {
                        false
                    };
                    let name = name.to_string_lossy().into_owned();
                    self.rename_file(&file, &name).await;
                } else {
                    return Err(FsError::Forbidden);
                }
            } else {
                // move
                let file = self
                    .get_file(from.clone())
                    .await?
                    .ok_or(FsError::NotFound)?;
                is_dir = if file.kind == 0 {
                    true
                } else {
                    false
                };
                let to_parent_file = self
                    .get_file(to.parent().unwrap().to_path_buf())
                    .await?
                    .ok_or(FsError::NotFound)?;
                let new_name = to_dav.file_name();
                self.move_file(&file, &to_parent_file.id).await;
            }

            if is_dir {
                self.dir_cache.invalidate(&from).await;
            }
            self.dir_cache.invalidate_parent(&from).await;
            self.dir_cache.invalidate_parent(&to).await;
            Ok(())
        }
        .boxed()
    }


    fn copy<'a>(&'a self, from_dav: &'a DavPath, to_dav: &'a DavPath) -> FsFuture<()> {
        let from = self.normalize_dav_path(from_dav);
        let to = self.normalize_dav_path(to_dav);
        debug!(from = %from.display(), to = %to.display(), "fs: copy");
        async move {

            let parent_path = to.parent().unwrap();

            let path_str = parent_path.to_string_lossy().into_owned();
            let parent_file = match self.get_by_path(&path_str).await{
                Ok(res)=>res.unwrap(),
                Err(err)=>{
                    error!("获取上级目录信息失败: {:?}", err);
                    panic!("获取上级目录信息失败: {:?}", err)
                }
            };
            if parent_file.id=="0" && parent_path.to_path_buf().to_string_lossy()=="/"{
                error!("根目录的文件夹无法修改或删除");
                panic!("根目录的文件夹无法修改或删除")
            }

            let file = self
                .get_file(from.clone())
                .await?
                .ok_or(FsError::NotFound)?;
            let to_parent_file = self
                .get_file(to.parent().unwrap().to_path_buf())
                .await?
                .ok_or(FsError::NotFound)?;
            let new_name = to_dav.file_name();
            self.copy_file(&file, &to_parent_file.id).await;
            self.dir_cache.invalidate(&to).await;
            self.dir_cache.invalidate_parent(&to).await;
            Ok(())
        }
        .boxed()
    }



    fn get_quota(&self) -> FsFuture<(u64, Option<u64>)> {
        async move {
            let (used, total) = self.get_useage_quota().await.map_err(|err| {
                error!(error = %err, "get quota failed");
                FsError::GeneralFailure
            })?;
            Ok((used, Some(total)))
        }
        .boxed()
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<Box<dyn DavMetaData>> {
        let path = self.normalize_dav_path(path);
        debug!(path = %path.display(), "fs: metadata");
        async move {
            let file = self.get_file(path).await?.ok_or(FsError::NotFound)?;
            Ok(Box::new(file) as Box<dyn DavMetaData>)
        }
        .boxed()
    }


    fn have_props<'a>(
        &'a self,
        _path: &'a DavPath,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = bool> + Send + 'a>> {
        Box::pin(ready(true))
    }

    fn get_prop(&self, dav_path: &DavPath, prop:DavProp) -> FsFuture<Vec<u8>> {
        let path = self.normalize_dav_path(dav_path);
        let prop_name = match prop.prefix.as_ref() {
            Some(prefix) => format!("{}:{}", prefix, prop.name),
            None => prop.name.to_string(),
        };
        debug!(path = %path.display(), prop = %prop_name, "fs: get_prop");
        async move {
            if prop.namespace.as_deref() == Some("http://owncloud.org/ns")
                && prop.name == "checksums"
            {
                let file = self.get_file(path).await?.ok_or(FsError::NotFound)?;
                if let sha1 = file.sha1.unwrap() {
                    let xml = format!(
                        r#"<?xml version="1.0"?>
                        <oc:checksums xmlns:d="DAV:" xmlns:nc="http://nextcloud.org/ns" xmlns:oc="http://owncloud.org/ns">
                            <oc:checksum>sha1:{}</oc:checksum>
                        </oc:checksums>
                    "#,
                        sha1
                    );
                    return Ok(xml.into_bytes());
                }
            }
            Err(FsError::NotImplemented)
        }
        .boxed()
    }





}

#[derive(Debug, Clone)]
struct UploadState {
    size: u64,
    buffer: BytesMut,
    chunk_count: u64,
    chunk: u64,
    upload_id: String,
    oss_args: Option<OssArgs>,
    sha1: Option<String>,
}

impl Default for UploadState {
    fn default() -> Self {
        Self {
            size: 0,
            buffer: BytesMut::new(),
            chunk_count: 0,
            chunk: 1,
            upload_id: String::new(),
            oss_args: None,
            sha1: None,
        }
    }
}

#[derive(Clone)]
struct FastDavFile {
    fs: WebdavDriveFileSystem,
    file: WebdavFile,
    parent_file_id: String,
    parent_dir: PathBuf,
    current_pos: u64,
    download_url: Option<String>,
    upload_state: UploadState,
}

impl Debug for FastDavFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastDavFile")
            .field("file", &self.file)
            .field("parent_file_id", &self.parent_file_id)
            .field("current_pos", &self.current_pos)
            .field("upload_state", &self.upload_state)
            .finish()
    }
}

impl FastDavFile {
    fn new(fs: WebdavDriveFileSystem, file: WebdavFile, parent_file_id: String,parent_dir: PathBuf,size: u64,sha1: Option<String>,) -> Self {
        Self {
            fs,
            file,
            parent_file_id,
            parent_dir,
            current_pos: 0,
            upload_state: UploadState {
                size,
                sha1,
                ..Default::default()
            },
            download_url: None,
        }
    }


    async fn get_download_url(&self,parent_dir: &PathBuf) -> Result<String, FsError> {
        debug!("get_download_url from cache or request");
        match &self.download_url {
            None=> { 
                debug!("下载地址为NONE第一次请求");
                self.fs.get_download_url(parent_dir,&self.file.id).await.map_err(|err| {
                    error!(file_id = %self.file.id, file_name = %self.file.name, error = %err, "get download url failed");
                    FsError::GeneralFailure
                })
             },
             Some(url) => { 
                debug!(url=%url,"下载地址不为NONE判断是否过期");
                if (is_url_expired(&url)) {
                    debug!(url=%url,"下载地址过期重新请求");
                    self.fs.get_download_url(parent_dir,&self.file.id).await.map_err(|err| {
                        error!(file_id = %self.file.id, file_name = %self.file.name, error = %err, "get download url failed");
                        FsError::GeneralFailure
                    })
                }else {
                    debug!(url=%url,"下载地址不过期直接返回");
                    Ok(url.to_string())
                }
             }
        } 
    }

    async fn prepare_for_upload(&mut self) -> Result<bool, FsError> {
        if self.upload_state.chunk_count == 0 {
            let size = self.upload_state.size;
            debug!(file_name = %self.file.name, size = size, "prepare for upload");

            if !self.file.id.is_empty() {
                // if let content_hash = self.file.clone().sha1.unwrap() {
                //     if let Some(sha1) = self.upload_state.sha1.as_ref() {
                //         if content_hash.eq_ignore_ascii_case(sha1) {
                //             debug!(file_name = %self.file.name, sha1 = %sha1, "skip uploading same content hash file");
                //             return Ok(false);
                //         }
                //     }
                // }

                if self.fs.skip_upload_same_size && self.file.size.parse::<u64>().unwrap() == size {
                    debug!(file_name = %self.file.name, size = size, "skip uploading same size file");
                    return Ok(false);
                }
                // existing file, delete before upload
                if let Err(err) = self
                    .fs
                    .remove_file(&self.file)
                    .await
                {
                    error!(file_name = %self.file.name, error = %err, "delete file before upload failed");
                }
            }
            // TODO: create parent folders?

        
            debug!("uploading {} ({} bytes)...", self.file.name, size);
            if size>0 {
                let hash = &self.file.clone().sha1.unwrap();
                let res: std::result::Result<UploadInitResponse, anyhow::Error> = self
                    .fs
                    .create_file_with_proof(&self.file.clone().provider.unwrap(),&self.file.name, &self.parent_file_id, hash, size)
                    .await;

                let upload_response: UploadInitResponse = match res {
                    Ok(upload_response_info) => upload_response_info,
                    Err(err) => {
                        error!(file_name = %self.file.name, error = %err, "create file with proof failed");
                        return Ok(false);
                    }
                };

                if upload_response.code != 200 as u64 {
                    error!(file_name = %self.file.name, error = upload_response.message);
                    return Ok(false);
                }

                let upload_buffer_size = self.fs.upload_buffer_size as u64;
                let chunk_count = size / upload_buffer_size + if size % upload_buffer_size != 0 { 1 } else { 0 };
                self.upload_state.chunk_count = chunk_count;
               

                let oss_args: OssArgs = match upload_response.extra {
                    Some(res)=>{
                        OssArgs {
                            uploader:upload_response.data.uploader,
                            sha1:upload_response.data.fileSha1,
                            chunkSize:upload_response.data.chunkSize,
                            extra_init:Some(res),
                            extra_last:None
                        }
                    },
                    None=>{
                        OssArgs {
                            uploader:upload_response.data.uploader,
                            sha1:upload_response.data.fileSha1,
                            chunkSize:upload_response.data.chunkSize,
                            extra_init:None,
                            extra_last:None
                        }
                    }
                };

                self.upload_state.oss_args = Some(oss_args);
    
                let oss_args = self.upload_state.oss_args.as_ref().unwrap();
                let pre_upload_info = self.fs.get_pre_upload_info(&oss_args).await;
                if let Err(err) = pre_upload_info {
                    error!(file_name = %self.file.name, error = %err, "get pre upload info failed");
                    return Ok(false);
                }
               
                self.upload_state.upload_id = match pre_upload_info {
                    Ok(upload_id) => upload_id,
                    Err(err) => {
                        error!(file_name = %self.file.name, error = %err, "get pre upload info failed");
                        return Ok(false);
                    }
                };
                debug!(file_name = %self.file.name, upload_id = %self.upload_state.upload_id, "pre upload info get upload_id success");
            }
        }
        Ok(true)
    }

    async fn maybe_upload_chunk(&mut self, remaining: bool) -> Result<(), FsError> {
        let chunk_size = if remaining {
            // last chunk size maybe less than upload_buffer_size
            self.upload_state.buffer.remaining()
        } else {
            self.fs.upload_buffer_size
        };
        let current_chunk = self.upload_state.chunk;

        debug!("chunk_size is {}",&chunk_size);

        if chunk_size > 0
            && self.upload_state.buffer.remaining() >= chunk_size
            && current_chunk <= self.upload_state.chunk_count
        {
            let chunk_data = self.upload_state.buffer.split_to(chunk_size);
            debug!(
                file_id = %self.file.id,
                file_name = %self.file.name,
                size = self.upload_state.size,
                "upload part {}/{}",
                current_chunk,
                self.upload_state.chunk_count
            );
            let upload_data = chunk_data.freeze();
            let mut oss_args = match self.upload_state.oss_args.clone() {
                Some(oss_args) => oss_args,
                None => {
                    error!(file_name = %self.file.name, "获取文件上传信息错误");
                    return Err(FsError::GeneralFailure);
                }
            };
            let res = self.fs.upload_chunk(&self.file,&oss_args,&self.upload_state.upload_id,current_chunk,upload_data.clone()).await;
            
            let part = match res {
                Ok(part) => part,
                Err(err) => {
                    error!(file_name = %self.file.name, error = %err, "上传分片失败，无法获取上传信息");
                    return Err(FsError::GeneralFailure);
                }
            };

            
            self.upload_state.oss_args = match part.clone().extra {
                Some(res) => {
                    let mut t = oss_args.clone();
                    t.extra_last = Some(res);
                    Some(t)
                },
                None=>(Some(oss_args.clone()))
            };


            debug!("文件上传结果:{:?}",part);
            debug!(chunk_count = %self.upload_state.chunk_count, current_chunk=current_chunk, "upload chunk info");
            if current_chunk == self.upload_state.chunk_count{
                debug!(file_name = %self.file.name, "upload finished");
                let mut buffer = Vec::new();
                let mut ser = XmlSerializer::with_root(Writer::new_with_indent(&mut buffer, b' ', 4), Some("CompleteMultipartUpload"));
                //self.upload_state.upload_tags.serialize(&mut ser).unwrap();
                let upload_tags = String::from_utf8(buffer).unwrap();
                self.fs.complete_upload(&self.file,upload_tags,&oss_args,&self.upload_state.upload_id).await;
                self.upload_state = UploadState::default();
                // self.upload_state.buffer.clear();
                // self.upload_state.chunk = 0;
                self.fs.dir_cache.invalidate(&self.parent_dir).await;
                info!("parent dir is  {} parent_file_id is {}", self.parent_dir.to_string_lossy().to_string(), &self.parent_file_id.to_string());
                self.fs.list_files_and_cache(self.parent_dir.to_string_lossy().to_string(), self.parent_file_id.to_string());
            }
            self.upload_state.chunk += 1;
        }
        Ok(())
    }

}

impl DavFile for FastDavFile {
    fn metadata(&'_ mut self) -> FsFuture<'_, Box<dyn DavMetaData>> {
        debug!(file_id = %self.file.id, file_name = %self.file.name, "file: metadata");
        async move {
            let file = self.file.clone();
            Ok(Box::new(file) as Box<dyn DavMetaData>)
        }
        .boxed()
    }

    fn write_buf(&'_ mut self, buf: Box<dyn Buf + Send>) -> FsFuture<'_, ()> {
        debug!(file_id = %self.file.id, file_name = %self.file.name, "file: write_buf");
        async move {
            if self.prepare_for_upload().await? {
                self.upload_state.buffer.put(buf);
                self.maybe_upload_chunk(false).await?;
            }
            Ok(())
        }
        .boxed()
    }

    fn write_bytes(&mut self, buf: Bytes) -> FsFuture<()> {
        debug!(file_id = %self.file.id, file_name = %self.file.name, "file: write_bytes");
        async move {
            if self.prepare_for_upload().await? {
                self.upload_state.buffer.extend_from_slice(&buf);
                self.maybe_upload_chunk(false).await?;
            }
            Ok(())
        }
        .boxed()
    }

    fn flush(&mut self) -> FsFuture<()> {
        debug!(file_id = %self.file.id, file_name = %self.file.name, "file: flush");
        async move {
            if self.prepare_for_upload().await? {
                self.maybe_upload_chunk(true).await?;
                self.fs.remove_uploading_file(&self.parent_file_id, &self.file.name);
                self.fs.dir_cache.invalidate(&self.parent_dir).await;
            }
            Ok(())
        }
        .boxed()
    }

    fn read_bytes(&mut self, count: usize) -> FsFuture<Bytes> {
        debug!(
            file_id = %self.file.id,
            file_name = %self.file.name,
            pos = self.current_pos,
            download_url = self.download_url,
            count = count,
            parent_id = %self.parent_file_id,
            "file: read_bytes",
        );
        async move {
            if self.file.id.is_empty() {
                // upload in progress
                return Err(FsError::NotFound);
            }
        
            let download_url = self.download_url.take();
            let download_url = if let Some(mut url) = download_url {
                if is_url_expired(&url) {
                    debug!(url = %url, "下载地址已经过期重新请求");
                    url = self.get_download_url(&self.parent_dir).await?;
                }
                url
            } else {
                debug!("获取文件的下载地址");
                self.get_download_url(&self.parent_dir).await?
            };

            
            let content = self
                .fs
                .download(&download_url,self.file.clone().play_headers, self.current_pos, count)
                .await
                .map_err(|err| {
                    error!(url = %download_url, error = %err, "download file failed");
                    FsError::NotFound
                })?;
            self.current_pos += content.len() as u64;
            self.download_url = Some(download_url);
            Ok(content)
        }
        .boxed()
    }

    fn seek(&mut self, pos: SeekFrom) -> FsFuture<u64> {
        debug!(
            file_id = %self.file.id,
            file_name = %self.file.name,
            pos = ?pos,
            "file: seek"
        );
        async move {
            let new_pos = match pos {
                SeekFrom::Start(pos) => pos,
                SeekFrom::End(pos) => (self.file.size.parse::<u64>().unwrap() as i64 - pos) as u64,
                SeekFrom::Current(size) => self.current_pos + size as u64,
            };
            self.current_pos = new_pos;
            Ok(new_pos)
        }
        .boxed()
    }

   
}

fn is_url_expired(url: &str) -> bool {
    debug!(url=url,"is_url_expired:");
    if let Ok(oss_url) = ::url::Url::parse(url) {
        let expires = oss_url.query_pairs().find_map(|(k, v)| {
            if k == "x-oss-expires" {
                if let Ok(expires) = v.parse::<u64>() {
                    return Some(expires);
                }
            }
            None
        });
        if let Some(expires) = expires {
            let current_ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            // 预留 1s
            return current_ts >= expires - 1;
        }
    }
    false
}



fn sign(token: &str, cid: &str, time: i64) -> String {
    let s = format!("{}{}{}", token, cid, time);
    let haser1 = to_md5(&s);
    let haser2 = to_md5(&haser1);
    haser2
}

fn to_md5(param_string: &str) -> String {
    let mut string_buffer = String::new();
    let mut hasher = Md5::new();
    hasher.update(param_string.as_bytes());
    let array_of_byte = hasher.finalize();
    for b in array_of_byte.iter() {
        let b1 = *b as i32;
        let i = if b1 < 0 { b1 + 256 } else { b1 };
        let _ = write!(string_buffer, "{:02x}", i);
    }
    string_buffer
}

fn get_time_in_millis(i: i64, i2: i64) -> i64 {
    let now = SystemTime::now();
    let seconds:u64 = (i * 86400 + i2 * 3600) as u64;
    let duration = std::time::Duration::new(seconds, 0);
    let new_time = now + duration;
    let since_epoch = new_time.duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as i64
}


fn get_file_sha1(body: Bytes) -> String {
    let mut hasher = Sha1::default();
    hasher.update(body);
    // let hash_code = hasher.finalize();
    //let file_hash = format!("{:X}",&hash_code);
    let result = hasher.finalize();
    let mut result_string = String::new();
    for byte in result.iter() {
        write!(result_string, "{:02x}", byte).unwrap();
    }
    result_string
}


fn get_file_type(file_name: &str) -> i32 {
    let mut map = HashMap::new();
    map.insert("txt", 1);
    map.insert("jpeg", 2);
    map.insert("jpg", 2);
    map.insert("gif", 2);
    map.insert("bmp", 2);
    map.insert("png", 2);
    map.insert("avif", 2);
    map.insert("heic", 2);
    map.insert("mp4", 3);
    map.insert("mkv", 3);
    map.insert("m4u", 3);
    map.insert("m4v", 3);
    map.insert("mov", 3);
    map.insert(".3gp", 3);
    map.insert("asf", 3);
    map.insert("avi", 3);
    map.insert("wmv", 3);
    map.insert("flv", 3);
    map.insert("mpe", 3);
    map.insert("mpeg", 3);
    map.insert("mpg", 3);
    map.insert("mpg4", 3);
    map.insert("mpeg4", 3);
    map.insert("mpga", 3);
    map.insert("rmvb", 3);
    map.insert("rm", 3);
    map.insert("aac", 4);
    map.insert("ogg", 4);
    map.insert("wav", 4);
    map.insert("wma", 4);
    map.insert("m3u", 4);
    map.insert("m4a", 4);
    map.insert("m4b", 4);
    map.insert("m4p", 4);
    map.insert("m4r", 4);
    map.insert("mp2", 4);
    map.insert("mp3", 4);
    map.insert("bin", 5);
    map.insert("class", 5);
    map.insert("conf", 5);
    map.insert("cpp", 5);
    map.insert("c", 5);
    map.insert("exe", 5);
    map.insert("gtar", 5);
    map.insert("gz", 5);
    map.insert("h", 5);
    map.insert("htm", 5);
    map.insert("html", 5);
    map.insert("jar", 5);
    map.insert("java", 5);
    map.insert("js", 5);
    map.insert("log", 5);
    map.insert("mpc", 5);
    map.insert("msg", 5);
    map.insert("pps", 5);
    map.insert("prop", 5);
    map.insert("rc", 5);
    map.insert("rtf", 5);
    map.insert("sh", 5);
    map.insert("tar", 5);
    map.insert("tgz", 5);
    map.insert("wps", 5);
    map.insert("xml", 5);
    map.insert("z", 5);
    map.insert("zip", 5);
    map.insert("apk", 5);
    map.insert("exe", 5);
    map.insert("ipa", 5);
    map.insert("app", 5);
    map.insert("hap", 5);
    map.insert("docx", 6);
    map.insert("doc", 6);
    map.insert("xls", 7);
    map.insert("xlsx", 7);
    map.insert("ppt", 8);
    map.insert("pptx", 8);
    map.insert("pdf", 9);
    map.insert("epub", 11);
    let file_ext = get_file_extension_name(file_name);
    match map.get(&file_ext as &str) {
        Some(file_type) => *file_type,
        None => 5,
    }
}

fn get_file_extension_name(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(index) => file_name[index + 1..].to_string(),
        None => "".to_string(),
    }
}
