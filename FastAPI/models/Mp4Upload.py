import json, requests
import time
import datetime
import re
import math
import hashlib
from cachelib import SimpleCache
from fastapi import HTTPException
import os
import sys
import configparser
sys.path.append(os.path.abspath('../'))
from schemas.schemas import *


class Mp4Upload():
    '''
    https://www.mp4upload.com/:mp4upload - Easy Way to Share your Videos
    '''
    def __init__(self,provider='',token=''):
        '''
        :param provider: 模型实例名称
        :param token: API token【官方叫key】可在https://www.mp4upload.com/account?op=my_account源码里获得
        '''
        # 创建配置文件对象
        self.config = configparser.SafeConfigParser()
        self.provider = provider
        self.token = token
        self.cache = SimpleCache()
        # 防止请求过于频繁，用于请求间隔时间
        self.sleep_time = 0.005
        # 缓存结果时间默认10分钟
        self.cache_time = 600
        self.headers = {
            "user-agent":"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_7_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/27.0.1453.93 Safari/537.36",
            "content-type":"application/json;charset=UTF-8",
        }
       
    # 文件列表方法 返回DavFile列表 请求内容为ListRequest，默认根目录ID为root
    def list_files(self, list_req:ListRequest):
        folderId=list_req.parent_file_id
        if folderId=='root':
            folderId=0
        file_list = self.cache.get(f"Mp4Upload-{self.token}-{folderId}")
        # 如果缓存中没有结果，则重新请求并缓存结果
        if not file_list:
            file_list = []
            url = f"https://www.mp4upload.com/api/folder/list?key={self.token}&fld_id={folderId}"
            try:
                response = requests.get(url, verify=False, headers=self.headers, timeout=100)
                # 如果请求失败，则抛出异常
            except requests.exceptions.RequestException as e:
                print("无法获取文件信息")
            result = json.loads(response.text)
            if result['msg']!='OK':
                raise HTTPException(status_code=400, detail="无法获取文件列表")

            for child in result['result']['folders']:
                file=child
                filesize = 0
                # 格式化时间为字符串
                # dt = datetime.datetime.fromtimestamp(file['uploaded'])
                # formatted_time = dt.strftime("%Y-%m-%d %H:%M:%S")
                formatted_time = file['uploaded']
                dav_file = DavFile(id=file['file_code'],provider=self.provider,parent_id=file['fld_id'],kind= 0,name=file['name'],size=str(filesize),create_time=formatted_time) 
                file_list.append(dav_file)

            file_folder={}
            for fld in result['result']['files']:
                file_folder[fld['file_code']]=fld['fld_id']

            file_ids = self.pluck(result['result']['files'],'file_code')
            ids = ','.join(map(str, file_ids))
            ids_url = f"https://www.mp4upload.com/api/file/info?key={self.token}&file_code="+ids
            try:
                ids_response = requests.get(ids_url, verify=False, headers=self.headers, timeout=100)
                # 如果请求失败，则抛出异常
            except requests.exceptions.RequestException as e:
                print("无法获取文件信息")
            ids_result = json.loads(ids_response.text)
            if ids_result['msg']!='OK':
                raise HTTPException(status_code=400, detail="无法获取文件列表")
                
            for mp4file in ids_result['result']:
                file=mp4file
                filesize = file['size']
                # 格式化时间为字符串
                formatted_time = file['uploaded']
                dav_file = DavFile(id=file['filecode'],provider=self.provider,parent_id=file_folder[file['filecode']],kind=1,name=file['name'],size=str(filesize),create_time=formatted_time) 
                file_list.append(dav_file)

            self.cache.set(f"Mp4Upload-{self.token}-{folderId}", file_list, timeout=self.cache_time)
        return file_list

    # 文件下载地址 返回下载地址
    def get_url(self,dav_file:DavFile):
        #这个url已经在列表页面得到，不需要再请求
        url = f"https://www.mp4upload.com/embed-{dav_file.file_id}.html"
        try:
            response = requests.get(url, verify=False, headers=self.headers, timeout=100)
            # 如果请求失败，则抛出异常
        except requests.exceptions.RequestException as e:
            raise HTTPException(status_code=400, detail="无法打开文件播放页面")
        parten="src: \"(.*)\""
        download_url = re.findall(parten, response.text)[0]
        #设置三小时后过期
        current_timestamp_sec = round(time.time())
        expires_timestamp_sec = current_timestamp_sec+10800
        if '?' in download_url:
            download_url=f"{download_url}&x-oss-expires={expires_timestamp_sec}"
        else:
            download_url=f"{download_url}?x-oss-expires={expires_timestamp_sec}"     

        return download_url


    def create_folder(self,create_folder_req:CreateFolderRequest):
        now = datetime.datetime.now()
        # 格式化时间为字符串
        formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
        folderId = create_folder_req.parent_id
        if folderId=='root':
            folderId='0'
        response = requests.get(f"https://www.mp4upload.com/api/folder/create?key={self.token}&parent_id={folderId}&name={create_folder_req.name}",verify=False, headers=self.headers, data=json.dumps(payload))
        result = json.loads(response.text)
        if result['msg']!='OK':
            raise HTTPException(status_code=400, detail="无法创建文件夹")
        if result['code']==200:
            self.cache.delete(f"Mp4Upload-{self.token}-{folderId}")
            dav_file = DavFile(id=result['fld_id'],parent_id=create_folder_req.parent_id,provider=create_folder_req.parend_file.provider,kind=0,name=create_folder_req.name,size='0',create_time=formatted_time)
            return dav_file
        else:
            raise HTTPException(status_code=400, detail="无法创建文件夹")



    def rename_file(self,rename_file_req:RenameFileRequest):
        folderId = rename_file_req.dav_file.parent_id
        if folderId=='root':
            folderId='0'
        response = requests.get(f"https://www.mp4upload.com/api/folder/rename?key={self.token}&fld_id={rename_file_req.dav_file.file_id}&name={rename_file_req.new_name}",verify=False, headers=self.headers, data=json.dumps(payload))
        result = json.loads(response.text)
        if result['msg']!='OK':
            raise HTTPException(status_code=400, detail="无法重命名文件")
        if result['code']==200:
            self.cache.delete(f"Mp4Upload-{self.token}-{folderId}")
            return rename_file_req.dav_file
        else:
            raise HTTPException(status_code=400, detail="无法重命名文件")



    # 辅助方法
    def pluck(self,lst, key):
        return [x.get(key) for x in lst]
