import json, requests
import time
import datetime
from datetime import timedelta
from datetime import datetime
from fastapi import HTTPException
import re
import math
import hashlib
from cachelib import SimpleCache
import os
import sys
import configparser
import base64
sys.path.append(os.path.abspath('../'))
from schemas.schemas import *
from webdav3.client import Client
from urllib.parse import urlparse

class WebDAV():
    '''
    WebDAV:WebDAV
    '''
    def __init__(self,provider='',url='',username='',password=''):
        '''
        :param provider: 模型实例名称
        :param url: webdav地址
        :param username: 登陆用户名
        :param password: 登陆密码
        '''
        # 创建配置文件对象
        self.provider = provider
        self.url = url
        self.username = username
        self.password = password
        self.cache = SimpleCache()
        # 防止请求过于频繁，用于请求间隔时间
        self.sleep_time = 0.005
        # 缓存结果时间默认10分钟
        self.cache_time = 600
        auth_token = base64.b64encode(f"{self.username}:{self.password}".encode('utf-8')).decode('utf-8')
        self.headers = {
            "user-agent":"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_7_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/27.0.1453.93 Safari/537.36",
            "Authorization": 'Basic '+ auth_token
        }
        # self.client = Client({
        #     'webdav_hostname': self.url,
        #     'webdav_login': self.username,
        #     'webdav_password': self.password
        # })
        # self.client.verify = False

        parsed_url = urlparse(self.url)
        self.netloc = parsed_url.netloc
        self.hostname = parsed_url.hostname
        self.path = parsed_url.path
        self.scheme = parsed_url.scheme


    # 文件列表方法 返回DavFile列表 请求内容为ListRequest，默认根目录ID为root
    def list_files(self, list_req:ListRequest):
        # 计算请求路径 
        path_str = list_req.path_str
        if list_req.parent_file_id=='root':
            path_str=self.path
        else:
            start_index=list_req.path_str.find('/',1)
            path_str=self.path+list_req.path_str[start_index:]

        file_list = self.cache.get(f"{self.username}-files-{list_req.path_str}")
        # 如果缓存中没有结果，则重新请求并缓存结果
        if not file_list:
            file_list = []
            client = Client({
                'webdav_hostname': self.scheme+"://"+self.netloc,
                'webdav_login': self.username,
                'webdav_password': self.password,
                'webdav_root': path_str,
             })
            files = client.list(get_info=True)
            for file in files:
                file['name'] = file['path'].split('/')[-2]

                if file['path']==self.path+'/':
                    continue

                kind = 0
                filesize = 0
                download_url = None
                now = datetime.now()
                # 格式化时间为字符串
                formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
                if file['modified'] is not None:
                    #dt = datetime.strptime(file['modified'], '%Y-%m-%dT%H:%M:%SZ')
                    dt = datetime.strptime(file['modified'], '%a, %d %b %Y %H:%M:%S %Z')
                    formatted_time = dt.strftime("%Y-%m-%d %H:%M:%S")
                if file['etag'] is None:
                    file['etag'] = base64.b64encode(f"{file['name']}:{file['path']}".encode('utf-8')).decode('utf-8')
                if not file['isdir']:
                    kind = 1
                    filesize = file['size']
                    download_url = self.scheme+"://"+self.netloc+file['path']
                    file['name']=file['path'].split('/')[-1]
                playe_headers = json.dumps(self.headers)
                dav_file = DavFile(id=file['etag'],provider=self.provider,parent_id=list_req.parent_file_id,kind= kind,name=file['name'],size=str(filesize),create_time=formatted_time,download_url=download_url,play_headers=playe_headers) 
                file_list.append(dav_file)
            self.cache.set(f"{self.username}-files-{list_req.path_str}", file_list, timeout=self.cache_time)
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



    
   