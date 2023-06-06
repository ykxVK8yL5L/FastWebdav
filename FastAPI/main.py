import os, sys, datetime
import configparser
from fastapi import FastAPI,APIRouter,Request,Query,Path
from fastapi.middleware.cors import CORSMiddleware
from models import *
from schemas.schemas import *

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

    return router


for provider in providers:
    provider_router = create_provider_router(provider)
    app.include_router(provider_router)



@app.get("/",response_model=list[DavFile])
async def root():
    now = datetime.datetime.now()
    # 格式化时间为字符串
    formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
    files = []
    for provider in providers:
        name = config.get(provider,'name')
        file = DavFile(id='root',provider=provider,parent_id=0,kind= 0,name=name,size=0,create_time=formatted_time,download_url=None)
        files.append(file)
    return files
