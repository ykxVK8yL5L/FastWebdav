import json, requests
import time
from datetime import datetime
import re
import math
import hashlib
from cachelib import SimpleCache
import os
import sys
import configparser
sys.path.append(os.path.abspath('../'))
from schemas.schemas import *


class Meting():
    def __init__(self,provider='',server='',playlist_id=''):
        # 创建配置文件对象
        self.config = configparser.SafeConfigParser()
        self.provider = provider
        self.server = server
        self.playlist_id = playlist_id
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
            folderId=''
        file_list = self.cache.get(f"{self.server}-{self.playlist_id}-{folderId}")
        # 如果缓存中没有结果，则重新请求并缓存结果
        if not file_list:
            file_list = []
            loop_index=1
            kind = '1'
            url = f"https://meting.yany.ml/api?server={self.server}&type=playlist&id={self.playlist_id}"
            try:
                response = requests.get(url, verify=False, headers=self.headers, timeout=100)
                # 如果请求失败，则抛出异常
            except requests.exceptions.RequestException as e:
                print("无法获取歌单信息")
            
            result = json.loads(response.text)
            for file in result:
                #2021-11-30T09:12:48.820+08:00
                now = datetime.now()
                # 格式化时间为字符串
                formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
                name = file['title']+".mp3"

                download_url = file['url']
                    #设置三小时后过期
                current_timestamp_sec = round(time.time())
                expires_timestamp_sec = current_timestamp_sec+10800
                if '?' in download_url:
                    download_url=f"{download_url}&x-oss-expires={expires_timestamp_sec}"
                else:
                    download_url=f"{download_url}?x-oss-expires={expires_timestamp_sec}"

                dav_file = DavFile(id=loop_index,provider=self.provider,parent_id="root",kind= kind,name=name,size="1000000000",create_time=formatted_time,download_url=download_url) 
                file_list.append(dav_file)
                loop_index+=1
            self.cache.set(f"{self.server}-{self.playlist_id}-{folderId}", file_list, timeout=self.cache_time)
        return file_list

    # 文件下载地址 返回下载地址
    def get_url(self,dav_file:DavFile):
        #这个url已经在列表页面得到，不需要再请求
        return ""