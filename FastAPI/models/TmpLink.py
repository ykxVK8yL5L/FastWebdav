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
import hashlib
import configparser

TMP_FILE_API="https://tmp-api.vx-cdn.com/api_v2/file"
TMP_TOKEN_API="https://tmp-api.vx-cdn.com/api_v2/token"

class TmpLink():
    def __init__(self,provider='',token=''):
        self.config = configparser.SafeConfigParser()
        self.provider = provider
        self.token = token
        self.uid = ''
        self.cache = SimpleCache()
        # 防止请求过于频繁，用于请求间隔时间
        self.sleep_time = 0.005
        # 缓存结果时间默认10分钟
        self.cache_time = 600
        # 分片大小
        self.slice_size = 33554432
        self.headers = {
            'authority': 'tmp-api.vx-cdn.com',
            'accept-language': 'zh-CN,zh;q=0.9,en;q=0.8',
            'content-type': 'application/x-www-form-urlencoded; charset=UTF-8',
            'origin': 'https://tmp.link',
            'referer': 'https://tmp.link/?tmpui_page=/app&listview=workspace',
            'user-agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36',
            'Cookie': f"PHPSESSID={self.token}",
        }

        try:
            with open("configs/tmplink.ini") as f:
                self.config.read_file(f)
        except IOError:
            # 如果配置文件不存在，创建一个空的配置文件
            with open("configs/tmplink.ini", "w") as f:
                self.config.write(f)
                
        if self.config.has_option(self.token, 'token'):
            self.token = self.config.get(self.token, 'token')
            self.uid = self.config.get(self.token, 'uid')
        else:
            self.config.add_section(self.token)
            self.set_user()



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
        response = requests.post(TMP_FILE_API,verify=False, headers=self.headers, data=data)
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
        # 第一次请求创建文件，貌似没啥用只是提交一下
        prepare_data = {
            "sha1":init_file.sha1,
            "filename": init_file.name,
            "filesize": init_file.size,
            "model": "2",
            "mr_id": "0",
            "skip_upload": "0",
            "action": "prepare_v4",
            "token": self.token,
        }
        prepare_response = requests.post(TMP_FILE_API,verify=False, headers=self.headers, data=prepare_data)
        # 每次得到操作的验证码
        captcha = self.getToken()
        # 开始上传请求，这个是最主要操作，获取到上传的utoken 
        # 返回示例:
        # {
        #     "data": {
        #         "utoken": "xxxxxxxxxx",
        #         "uploader": "https:\/\/tmp-hd4.vx-cdn.com",
        #         "src": "42.224.203.231"
        #     },
        #     "status": 1,
        #     "debug": []
        # }
        upload_request_data = {
            "action":"upload_request_select2",
            "token": self.token,
            "filesize": init_file.size,
            "captcha": captcha,
        }
        upload_request_response = requests.post(TMP_FILE_API,verify=False, headers=self.headers, data=upload_request_data)
        result = json.loads(upload_request_response.text)
        init_data = InitResponseData(uploader=result['data']['uploader'],fileName=init_file.name,fileSize=init_file.size,fileSha1=init_file.sha1,chunkSize=self.slice_size)
        response = InitUploadResponse(code=200,message="文件已经上传",data=init_data,extra=result['data']['utoken'])
        return response

    # 文件分片上传
    def upload_chunk(self,slice_req:SliceUploadRequest,filedata:bytes):
        upload_url = slice_req.oss_args.uploader+"/app/upload_slice"
        # 获取准备信息 返回示例
        # {"status":3,"data":{"next":0,"total":2,"wait":2,"uploading":0,"success":0}}
        # uptoken为sha1加密：uid+file.name+file.size 但是目前官方网站有Bug
        sha1_str = str(self.uid)+slice_req.name+slice_req.dav_file.size
        # 创建sha1对象
        sha1 = hashlib.sha1()
        # 更新字符串
        sha1.update(sha1_str.encode('utf-8'))
        # 获取加密后的字符串
        uptoken = sha1.hexdigest()
        prepare_data = {
            'token': self.token,
            'uptoken': xxxxxxxxxxxxxxx,
            'action': prepare,
            'sha1': 0,
            'filename': slice_req.dav_file.name,
            'filesize': 38531346,
            'slice_size': self.slice_size,
            'utoken': slice_req.oss_args.extra_init,
            'mr_id': 0,
            'model': 2,
        }
        prepare_response = requests.post(upload_url,verify=False, headers=self.headers, data=upload_request_data,files=files)
        prepare_info = json.loads(prepare_response.text)
        # 获取操作验证码
        captcha = self.getToken()
        # 上传操作 返回示例
        # {"status":5,"data":"upload slice success"}
        upload_data = {
            "sha1": slice_req.dav_file.sha1,
            "index": prepare_info['data']['next'],
            "action": "upload_slice",
            "slice_size": slice_req.oss_args.slice_size,
            "captcha": captcha,
        }
        files = {'filedata': ('slice', filedata)}
        upload_response = requests.post(upload_url,verify=False, headers=self.headers, data=upload_request_data,files=files)
        result = json.loads(upload_response.text)
        code = 100;
        if result['status'] == 5:
            code=200
        upload_data = FileUploadInfo(fileName=slice_req.dav_file.name,fileSize=slice_req.dav_file.size,fileHash=slice_req.dav_file.sha1,chunkIndex=slice_req.current_chunk,chunkSize=slice_req.oss_args.chunkSize,uploadState=result['status'])
        response = SliceUploadResponse(code=code,message=result[data], data=upload_data)
        return response
    
    # 分片上传完成后的处理
    def complete_upload(self,complete_req:CompleteUploadRequest):
        response = CompleteUploadResponse(status=101,data="稍后实现")
        return response


    # 以下都是辅助方法
    def getToken(self):
        # 返回操作的验证码，示例
        # {
        #     "data": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        #     "status": 1,
        #     "debug": []
        # }
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
    
    def set_user(self) -> str:
        loop_index = 1
        token = ''
        uid = ''
        while True:
            payload = {
                'action': 'get_detail',
                'token': self.token,
            }
            response = requests.post("https://tmp-api.vx-cdn.com/api_v2/user",verify=False, headers=self.headers, data=payload)
            result = json.loads(response.text)
            if 'uid' not in result['data']:
                print(f"第{loop_index}次无法获取uid")
                uid = 'error'
            else:
                uid = result['data']['uid']
                break 
            if loop_index>2:
                break
            loop_index+=1

        if uid == 'error':
            print("无法获取token请稍后再试")
        else:
            self.config.set(self.token, 'token',self.token)
            self.config.set(self.token, 'uid',uid)
            with open('configs/tmplink.ini', 'w') as f:
                self.config.write(f)
            self.token = self.config.get(self.token, 'token')
            self.uid = self.config.get(self.token, 'uid')


