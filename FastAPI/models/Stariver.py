import json, requests
import time
import datetime
from datetime import timedelta
from fastapi import HTTPException
import re
import math
import hashlib
from cachelib import SimpleCache
import os
import sys
import configparser
sys.path.append(os.path.abspath('../'))
from schemas.schemas import *


class Stariver():
    '''
    Stariver:小龙云盘
    '''
    def __init__(self,provider='',token=''):
        '''
        :param provider: 模型实例名称
        :param token: 登陆密钥
        '''
        # 创建配置文件对象
        self.config = configparser.SafeConfigParser()
        self.provider = provider
        self.token = token
        self.key = ''
        self.uid = ''
        self.cache = SimpleCache()
        # 防止请求过于频繁，用于请求间隔时间
        self.sleep_time = 0.005
        # 缓存结果时间默认10分钟
        self.cache_time = 600
        self.headers = {
            "Host": "productapi.stariverpan.com",
            "accept": "application/json, text/plain, */*",
            "sec-fetch-dest":"empty",
            "client-platform":"mac",
            "custom-agent":"PC",
            "accept-language":"zh",
            "client-version":"3.2.7",
            "user-agent":"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) xiaolongyunpan/3.2.7 Chrome/100.0.4896.143 Electron/18.2.0 Safari/537.36",
            "content-type":"application/json;charset=UTF-8",
            "sec-fetch-site":"cross-site",
            "sec-fetch-mode":"cors",
            "authorization":"Bearer "+self.token,
            "Custom-Agent":"PC",
        }
        # 需要请求网络并获取key和用户id
        try:
            with open("configs/stariver.ini") as f:
                self.config.read_file(f)
        except IOError:
            # 如果配置文件不存在，创建一个空的配置文件
            with open("configs/stariver.ini", "w") as f:
                self.config.write(f)
        
        if self.config.has_option(self.token, 'key'):
            self.key = self.config.get(self.token, 'key')
            self.uid = self.config.get(self.token, 'uid')
        else:
            self.config.add_section(self.token)
            login_data = {'token':self.token}
            login_response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1/user/login",verify=False, headers=self.headers, data=json.dumps(login_data))
            login_result = json.loads(login_response.text)
            self.config.set(self.token, 'uid',login_result['data']['id'])
            encryption_response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1/user/encryption-key",verify=False, headers=self.headers)
            encryption_result = json.loads(encryption_response.text)
            self.config.set(self.token, 'key',encryption_result['data']['Key'])
            with open('configs/stariver.ini', 'w') as f:
                self.config.write(f)
            self.key = self.config.get(self.token, 'key')
            self.uid = self.config.get(self.token, 'uid')


    # 文件列表方法 返回DavFile列表 请求内容为ListRequest，默认根目录ID为root
    def list_files(self, list_req:ListRequest):
        folderId=list_req.parent_file_id
        if folderId=='root':
            folderId='0'
        file_list = self.cache.get(f"{self.token}-files-{folderId}")
        # 如果缓存中没有结果，则重新请求并缓存结果
        if not file_list:
            file_list = []
            loop_index=1;
            while True:
                payload = {
                    'fileType':[],
                    'fileName':"",
                    'pageNum':loop_index,
                    'pageSize':50,
                    'parentId':folderId,
                    'sortType':"desc",
                    'sortFlag':"upload",
                }
                response = requests.post("https://productapi.stariverpan.com/cloudfile/v1/all-files",verify=False, headers=self.headers, data=json.dumps(payload))
                result = json.loads(response.text)
                kind = 0
                download_url = None
                for file in result['data']['data']:
                    if file['isFolder'] != 1:
                        kind = 1
                        gateway = "https://ipfsgw01.stariverpan.com:9096/ipfs/";
                        cid = file['fileCid']
                        ts = self.get_time_in_millis(0,24)
                        t = int(ts / 1000)
                        s = self.sign(self.key,cid,t)
                        download_url = f"{gateway}{cid}?v=1&u={self.uid}&t={t}&s={s}"
                         #设置三小时后过期
                        current_timestamp_sec = round(time.time())
                        expires_timestamp_sec = current_timestamp_sec+10800
                        if '?' in download_url:
                            download_url=f"{download_url}&x-oss-expires={expires_timestamp_sec}"
                        else:
                            download_url=f"{download_url}?x-oss-expires={expires_timestamp_sec}"
                    dav_file = DavFile(id=file['id'],provider=self.provider,parent_id=file['id'],kind= kind,name=file['fileName'],size=file['fileSize'],create_time=file['createTime'],download_url=download_url) 
                    file_list.append(dav_file)
                time.sleep(self.sleep_time)
                if result['data']['totalPage']==0 or result['data']['totalPage']==loop_index:
                    break
                loop_index+=1
            self.cache.set(f"{self.token}-files-{folderId}", file_list, timeout=self.cache_time)
        return file_list

    # 文件下载地址 返回下载地址
    def get_url(self,dav_file:DavFile):
        #这个url已经在列表页面得到，不需要再请求保留添加过期注释供参考
        #设置三小时后过期
        # current_timestamp_sec = round(time.time())
        # expires_timestamp_sec = current_timestamp_sec+10800
        # download_url = result['data']
        # download_expires_url = ""
        # if '?' in download_url:
        #     download_expires_url=f"{download_url}&x-oss-expires={expires_timestamp_sec}"
        # else:
        #     download_expires_url=f"{download_url}?x-oss-expires={expires_timestamp_sec}"
        return ""

    def create_folder(self,create_folder_req:CreateFolderRequest):
        now = datetime.datetime.now()
        # 格式化时间为字符串
        formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
        folderId = create_folder_req.parent_id
        if folderId=='root':
            folderId='0'
        payload = {
            'parentId':folderId,
            'fileName':create_folder_req.name,
        }
        response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1.2/cloud/addFolder",verify=False, headers=self.headers, data=json.dumps(payload))
        result = json.loads(response.text)

        if result['code']==200:
            self.cache.delete(f"{self.token}-files-{folderId}")
            dav_file = DavFile(id='123',parent_id=create_folder_req.parent_id,provider=create_folder_req.parend_file.provider,kind=0,name="testcreate",size='0',create_time=formatted_time)
            return dav_file
        else:
            raise HTTPException(status_code=400, detail="无法创建文件夹")

    
    def remove_file(self,remove_file_req:RemoveFileRequest):
        folderId = remove_file_req.dav_file.parent_id
        if folderId=='root':
            folderId='0'
        payload = {
            'ids':[remove_file_req.dav_file.file_id],
        }
        response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1.2/cloud/fileLogicDel",verify=False, headers=self.headers, data=json.dumps(payload))
        result = json.loads(response.text)

        if result['code']==200:
            self.cache.delete(f"{self.token}-files-{folderId}")
            return remove_file_req.dav_file
        else:
            raise HTTPException(status_code=400, detail="无法创建文件夹")
    
    def rename_file(self,rename_file_req:RenameFileRequest):
        #官方不支持重命名，先放这
        # folderId = rename_file_req.dav_file.parent_id
        # if folderId=='root':
        #     folderId='0'
        # payload = {
        #     'ids':[rename_file_req.dav_file.file_id],
        # }
        # response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1.2/cloud/fileLogicDel",verify=False, headers=self.headers, data=json.dumps(payload))
        # result = json.loads(response.text)

        # if result['code']==200:
        #     self.cache.delete(f"{self.token}-files-{folderId}")
        #     return rename_file_req.dav_file
        # else:
        #     raise HTTPException(status_code=400, detail="无法创建文件夹")
        raise HTTPException(status_code=400, detail="暂不支持重命名文件")

    def move_file(self,move_file_req:MoveFileRequest):
        folderId = move_file_req.dav_file.parent_id
        if folderId=='root':
            folderId='0'
        payload = {
            'ids':[move_file_req.dav_file.file_id],
            'parentId':move_file_req.new_parent_id,
        }
        response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1.2/cloud/fileMove",verify=False, headers=self.headers, data=json.dumps(payload))
        result = json.loads(response.text)

        if result['code']==200:
            self.cache.delete(f"{self.token}-files-{folderId}")
            return move_file_req.dav_file
        else:
            raise HTTPException(status_code=400, detail="无法移动文件")
    
    def copy_file(self,copy_file_req:CopyFileRequest):
        #官方不支持重命名，先放这
        # folderId = copy_file_req.dav_file.parent_id
        # if folderId=='root':
        #     folderId='0'
        # payload = {
        #     'ids':[copy_file_req.dav_file.file_id],
        # }
        # response = requests.post("https://productapi.stariverpan.com/cmsprovider/v1.2/cloud/fileLogicDel",verify=False, headers=self.headers, data=json.dumps(payload))
        # result = json.loads(response.text)

        # if result['code']==200:
        #     self.cache.delete(f"{self.token}-files-{folderId}")
        #     return copy_file_req.dav_file
        # else:
        #     raise HTTPException(status_code=400, detail="无法创建文件夹")
        raise HTTPException(status_code=400, detail="暂不支持复制文件")

       


    # 以下都是辅助方法
    def sign(self,token: str, cid: str, time: int) -> str:
        s = token + cid + str(time)
        haser1 = self.to_md5(s)
        haser2 = self.to_md5(haser1)
        return haser2

    def to_md5(self,param_string: str) -> str:
        string_buffer = ""
        hasher = hashlib.md5(param_string.encode('utf-8'))
        array_of_byte = hasher.digest()
        for b in array_of_byte:
            b1 = b if b >= 0 else b + 256
            string_buffer += f"{b1:02x}"
        return string_buffer

    def get_time_in_millis(self,i: int, i2: int) -> int:
        now = datetime.datetime.now()
        seconds = i * 86400 + i2 * 3600
        duration = timedelta(seconds=seconds)
        new_time = now + duration
        since_epoch = new_time.timestamp()
        return int(since_epoch * 1000)
   
    
        
