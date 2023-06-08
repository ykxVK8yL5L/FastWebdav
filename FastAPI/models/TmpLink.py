import json, requests
import time
import re
import math
from cachelib import SimpleCache
from fastapi import Request
import os
import sys
sys.path.append(os.path.abspath('../'))
from schemas.schemas import *

TMP_FILE_API="https://tmp-api.vx-cdn.com/api_v2/file"
TMP_TOKEN_API="https://tmp-api.vx-cdn.com/api_v2/token"

class TmpLink():
    def __init__(self,provider='',token=''):
        self.provider = provider
        self.token = token
        self.cache = SimpleCache()
        # 防止请求过于频繁，用于请求间隔时间
        self.sleep_time = 0.005
        # 缓存结果时间默认10分钟
        self.cache_time = 600
        self.headers = {
            'authority': 'tmp-api.vx-cdn.com',
            'accept-language': 'zh-CN,zh;q=0.9,en;q=0.8',
            'content-type': 'application/x-www-form-urlencoded; charset=UTF-8',
            'origin': 'https://tmp.link',
            'referer': 'https://tmp.link/?tmpui_page=/app&listview=workspace',
            'user-agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36',
        }
    # 文件列表方法 返回DavFile列表 请求内容为ListRequest,默认根目录ID为root
    def list_files(self, list_req:ListRequest):
        folderId=list_req.parent_file_id
        file_list = self.cache.get(f"{self.token}-files-{folderId}")
        # 如果缓存中没有结果，则重新请求并缓存结果
        if not file_list:
            file_list = []
            fileinfo=self.getFileInfo()
            list_range=math.ceil(int(fileinfo['nums'])/50)
            for x in range(0, list_range):
                payload = {
                    'action': 'workspace_filelist_page',
                    'page': x,
                    'token': self.token,
                    'sort_type':'',
                    'sort_by':'',
                    'photo':0,
                    'search':'',
                }
                response = requests.post(TMP_FILE_API, verify=False,headers=self.headers, data=payload)
                result = json.loads(response.text)
                for file in result['data']:
                    dav_file = DavFile(id=file['ukey'],provider=self.provider,parent_id='root',kind= 1,name=file['fname'],size=file['fsize'],create_time=file['ctime']) 
                    file_list.append(dav_file)
                time.sleep(self.sleep_time)
            self.cache.set(f"{self.token}-files-{folderId}", file_list, timeout=self.cache_time)
        return file_list

    # 文件下载地址 返回下载地址
    def get_url(self,dav_file:DavFile):
        token = self.getToken()
        data = {
            'action': 'download_req',
            'ukey': dav_file.file_id,
            'token': self.token,
            'captcha': token,
        }
        response = requests.post('https://tmp-api.vx-cdn.com/api_v2/file',verify=False, headers=self.headers, data=data)
        result = json.loads(response.text)
        #设置三小时后过期
        current_timestamp_sec = round(time.time())
        expires_timestamp_sec = current_timestamp_sec+10800
        download_url = result['data']
        download_expires_url = ""
        if '?' in download_url:
            download_expires_url=f"{download_url}&x-oss-expires={expires_timestamp_sec}"
        else:
            download_expires_url=f"{download_url}?x-oss-expires={expires_timestamp_sec}"
        return download_expires_url
    
    # 初始化文件上传，如果不需要的话根据需要自己构造返回的InitUploadResponse
    def init_upload(self,init_file:InitUploadRequest):
        init_data = InitResponseData(uploader=f"http://127.0.0.1/{self.provider}/upload",fileName=init_file.name,fileSize=init_file.size,fileSha1=init_file.sha1,chunkSize=16777216)
        response = InitUploadResponse(code=200,message="文件已经上传",data=init_data)
        return response

    # 文件分片上传
    def upload_chunk(self,slice_req:SliceUploadRequest,filedata:bytes):
        upload_data = FileUploadInfo(fileName=slice_req.dav_file.name,fileSize=slice_req.dav_file.size,fileHash=slice_req.dav_file.sha1,chunkIndex=slice_req.current_chunk,chunkSize=slice_req.oss_args.chunkSize,uploadState=0)
        response = SliceUploadResponse(code=200,message="稍后实现", data=upload_data)
        return response
    
    # 分片上传完成后的处理
    def complete_upload(self,complete_req:CompleteUploadRequest):
        response = CompleteUploadResponse(status=101,data="稍后实现")
        return response


    # 以下都是辅助方法
    def getToken(self):
        payload = {
            'action': 'challenge',
            'token': self.token,
        }
        response = requests.post(TMP_TOKEN_API,verify=False, headers=self.headers, data=payload)
        result = json.loads(response.text)
        return result['data']

    def getFileInfo(self):
        data = {
            'action': 'total',
            'token': self.token,
        }
        response = requests.post(TMP_FILE_API,verify=False, headers=self.headers, data=data)
        result = json.loads(response.text)
        return result['data']

    
        
