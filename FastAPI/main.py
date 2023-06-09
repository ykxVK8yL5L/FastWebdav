import os, sys
from typing import Annotated
from datetime import datetime
import configparser
from fastapi import FastAPI,APIRouter,Request,Query,Path,Request,File,Form
from fastapi.middleware.cors import CORSMiddleware
from pydantic import parse_obj_as
from models import *
from schemas.schemas import *
import json
import base64

#app = FastAPI(docs_url=None, redoc_url=None)
app = FastAPI(title='FastWebdav的API',description='为webdav提供数据支持', redoc_url=None)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)


# 创建配置文件对象
config = configparser.SafeConfigParser()


# 读取配置文件
try:
    with open("configs/providers.ini") as f:
        config.read_file(f)
except IOError:
    # 如果配置文件不存在，创建一个空的配置文件
    with open("configs/providers.ini", "w") as f:
        config.write(f)

providers = config.sections()

def create_provider_router(name):
    router = APIRouter(prefix=f"/{name}",tags=[name],responses={404: {"description": "Not found"}})
    provider = eval(config.get(name, 'provider'))
    @router.post("/list")
    async def get_files(list_req:ListRequest)-> list[DavFile]:
        '''
        返回文件列表，请求需以json格式post过来，请勿以text请求，否则会报错
        '''
        return provider.list_files(list_req)

    @router.post("/url")
    async def get_url(dav_file: DavFile)-> str:
        '''
        返回文件的下载地址,有些可以在列表页算出来的就不需要请求了，可以添加?x-oss-expires=时间戳 来控制过期时间，如果rust的缓存时间先到以缓存时间为准
        '''
        return provider.get_url(dav_file)
    
    @router.post("/init")
    async def init_upload(init_file: InitUploadRequest)-> InitUploadResponse:
        '''
        返回创建文件的响应，具体看schemas.初始化文件分片上传，通常是向服务器请求创建文件返回一个上传地址分片大小等信息，由于各个服务需要不一样，不一定全部可用，不断完善吧
        '''
        return provider.init_upload(init_file)
    
    @router.post("/upload_chunk")
    async def upload_chunk(filedata: Annotated[bytes, File()],slice_req: Annotated[str, Form()])-> SliceUploadResponse:
        '''
        文件分片上传
        '''
        json_str = base64.b64decode(slice_req).decode('utf-8')
        slice_req_obj = parse_obj_as(SliceUploadRequest, json.loads(json_str))
        return provider.upload_chunk(slice_req_obj,filedata)


    @router.post("/complete_upload")
    async def complete_upload(complete_req:CompleteUploadRequest)-> CompleteUploadResponse:
        '''
        文件分片上传
        '''
        return provider.complete_upload(complete_req)


    return router


for provider in providers:
    provider_router = create_provider_router(provider)
    app.include_router(provider_router)



@app.get("/",response_model=list[DavFile])
async def root():
    now = datetime.now()
    # 格式化时间为字符串
    formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
    files = []
    for provider in providers:
        name = config.get(provider,'name')
        file = DavFile(id='root',provider=provider,parent_id=0,kind= 0,name=name,size=0,create_time=formatted_time,download_url=None)
        files.append(file)
    return files
