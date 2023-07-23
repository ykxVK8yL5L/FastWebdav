use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use futures_util::future::FutureExt;
use serde::{Deserialize, Serialize};
use dav_server::fs::{DavDirEntry, DavMetaData, FsFuture, FsResult};
use serde_json::Value;



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct LoginResponse {
    pub code: u32,
    pub message: String,
    pub submessage: String,
    pub data:LoginData,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct LoginRequest {
    pub token: String,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct LoginData {
    pub token: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateFolderRequest<'a> {
    pub name: &'a str,
    pub parent_id: &'a str,
    pub parend_file:WebdavFile,
    pub path_str: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoveFileRequest{
    pub file:WebdavFile,
    pub remove_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameFileRequest<'a>{
    pub file:WebdavFile,
    pub new_name:&'a str,
    pub from:String,
    pub to:String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveFileRequest<'a>{
    pub file:WebdavFile,
    pub new_parent_id:&'a str,
    pub from:String,
    pub to:String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopyFileRequest<'a>{
    pub file:WebdavFile,
    pub new_parent_id:&'a str,
}



#[derive(Debug, Clone, Serialize)]
pub struct DelFileRequest {
    pub ids: Vec<String>,
}



#[derive(Debug, Clone, Serialize)]
pub struct MoveTo {
    pub parent_id: String,
}

mod my_date_format {
    use chrono::{DateTime, Utc, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";
    pub fn serialize<S>(
        date: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}




#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}





#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Folder,
    File,
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct QuotaResponse {
    pub kind: String,
    pub expires_at: String,
    pub quota: Quota,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Quota {
    pub kind: String,
    pub limit: u64,
    pub usage: u64,
    pub usage_in_trash: u64,
    pub play_times_limit: u64,
    pub play_times_usage:u64,
}




#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct UploadInitRequest {
    pub provider: String,
    pub name: String,
    pub parent_file_id: String,
    pub sha1: String,
    pub size: u64,
}



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct UploadInitResponse {
    pub code: u64,
    pub message: String,
    pub extra: Option<String>,
    pub data: InitResponseData,
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct InitResponseData {
    pub uploader: String,
    pub fileName: String,
    pub fileSize: u64,
    pub chunkSize: u64,
    pub fileSha1: String,
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct ObjProvider {
    pub provider: String,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct OssArgs {
    pub uploader:String,
    pub sha1:String,
    pub chunkSize:u64,
    pub extra_init:Option<String>,
    pub extra_last:Option<String>,
}



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct CompleteFileUpload {
    pub data: FileUploadInfo,
    pub status:u64,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct AddFileResponse {
    pub code: u64,
    pub message: String,
    pub submessage: String,
    pub count: u64,
    pub stime: u64,
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct CompleteUploadRequest {
    pub file: WebdavFile,
    pub oss_args: OssArgs,
    pub upload_tags: String,
    pub upload_id: String,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct SliceUploadRequest {
    pub file:WebdavFile,
    pub oss_args:OssArgs,
    pub upload_id:String,
    pub current_chunk:u64,
}



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct SliceUploadResponse {
    pub code: u64,
    pub message: String,
    pub extra: Option<String>,
    pub data: FileUploadInfo,
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct FileUploadInfo {
    pub fileName: String,
    pub fileSize: u64,
    pub fileHash: String,
    pub chunkIndex: u64,
    pub chunkSize: u64,
    pub uploadState: u64,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct PartInfo {
    pub chunkIndex: u64,
    pub chunkSize: u64,
}

// #[derive(Debug, Serialize, Deserialize)]
// struct Example {
//     next: serde_json::Value,
// }



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct UploadResponse {
    pub upload_type: String,
    pub resumable: Resumable,
    pub file: WebdavFile,
}



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct PrepareFileResponse {
    pub data: PrepareInfo,
    pub status: u64,
}



#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct CompleteUploadResponse {
    pub data: String,
    pub status: u64,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareInfo {
    pub next: u64,
    pub total: u64,
    pub wait: u64,
    pub uploading: u64,
    pub success: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SliceNextResult {
    Bool(bool),
    Int(i64),
}


#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct FileResponse {
    pub data: String,
    pub status: u64,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub data: UploaderResponse,
    pub status: u64,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct UploaderResponse {
    pub utoken: String,
    pub uploader: String,
    pub src:String,
}








#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Resumable {
    pub kind: String,
    pub provider: String,
    pub params: UploadParams,
}

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct UploadParams {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub bucket: String,
    pub endpoint: String,
    pub expiration: String,
    pub key: String,
    pub security_token: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct InitiateMultipartUploadResult {
    pub Bucket: String,
    pub Key: String,
    pub UploadId: String,
}




#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct WebdavFile {
    pub id: String,
    pub provider:Option<String>,
    pub name: String,
    pub oriname: Option<String>,
    //#[serde(rename = "parentId")]
    pub parent_id: String,
    pub size: String,
    pub kind: u64,
    #[serde(with = "my_date_format")]
    pub create_time: DateTime<Utc>,
    pub download_url: Option<String>,
    pub sha1: Option<String>,
    pub play_headers: Option<String>,
    pub password: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesListRequest {
    pub path_str: Value,
    pub parent_file_id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesListResponse {
    pub data: FilesList,
    pub code: Value,
    pub ts: Value,
    pub stime: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesList {
    pub data: Vec<WebdavFile>,
    pub pageNum: Value,
    pub pageSize: Value,
    pub count: Value,
    pub totalPage: Value,
}




impl DavMetaData for WebdavFile {
    fn len(&self) -> u64 {
        //self.size
        self.size.parse::<u64>().unwrap()
    }

    fn modified(&self) -> FsResult<SystemTime> {
        let timestamp = self.create_time.timestamp();
        let system_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
        Ok(system_time)
    }

    fn is_dir(&self) -> bool {
        //matches!(self.kind, String::from("drive#folder") )
        self.kind==0
    }

    fn created(&self) -> FsResult<SystemTime> {
        let timestamp = self.create_time.timestamp();
        let system_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
        Ok(system_time)
    }
}

impl DavDirEntry for WebdavFile {
    fn name(&self) -> Vec<u8> {
        self.name.as_bytes().to_vec()
    }

    fn metadata(&self) -> FsFuture<Box<dyn DavMetaData>> {
        async move { Ok(Box::new(self.clone()) as Box<dyn DavMetaData>) }.boxed()
    }
}

impl WebdavFile {
    pub fn new_root() -> Self {
        Self {
            id: "0".to_string(),
            provider:None,
            kind:0,
            name: "".to_string(),
            oriname: None,
            parent_id: "".to_string(),
            size: "0".to_string(),
            create_time: chrono::offset::Utc::now(),
            download_url:None,
            sha1:None,
            play_headers:None,
            password:None,
        }
    }
}


