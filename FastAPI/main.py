import os,sys,glob
from typing import Annotated
from datetime import datetime
import configparser
from fastapi import FastAPI,APIRouter,Request,Query,Path,Request,File,Form
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from fastapi.responses import StreamingResponse,HTMLResponse,FileResponse
from fastapi.templating import Jinja2Templates
from pydantic import parse_obj_as
from models import *
from schemas.schemas import *
import json
import base64
import re


#app = FastAPI(docs_url=None, redoc_url=None)
app = FastAPI(title='FastWebdav的API',description='为webdav提供数据支持', redoc_url=None)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)
app.mount("/static", StaticFiles(directory="static"), name="static")
templates = Jinja2Templates(directory="templates")

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
        文件分片上传\n
        filedata: 上传的数据，为字节型。\n
        slice_req: form提交过来的数据，从中提取SliceUploadRequest模型。\n
        返回:SliceUploadResponse上传响应模型
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
    
    @router.post("/create_folder")
    async def create_folder(create_folder_req:CreateFolderRequest)-> DavFile:
        '''
        文件分片上传
        '''
        return provider.create_folder(create_folder_req)
    

    @router.post("/remove_file")
    async def remove_file(remove_file_req:RemoveFileRequest)-> DavFile:
        '''
        文件分片上传
        '''
        return provider.remove_file(remove_file_req)

    @router.post("/rename_file")
    async def rename_file(rename_file_req:RenameFileRequest)-> DavFile:
        '''
        文件分片上传
        '''
        return provider.rename_file(rename_file_req)
    
    @router.post("/move_file")
    async def move_file(move_file_req:MoveFileRequest)-> DavFile:
        '''
        文件分片上传
        '''
        return provider.move_file(move_file_req)
    
    @router.post("/copy_file")
    async def copy_file(copy_file_req:CopyFileRequest)-> DavFile:
        '''
        文件分片上传
        '''
        return provider.copy_file(copy_file_req)

    return router


for provider in providers:
    provider_router = create_provider_router(provider)
    app.include_router(provider_router)



@app.get("/",response_model=list[DavFile],summary='所有provider',description='获取所有provider,就当是根目录文件夹吧')
async def root():
    now = datetime.now()
    # 格式化时间为字符串
    formatted_time = now.strftime("%Y-%m-%d %H:%M:%S")
    files = []
    for provider in providers:
        name = config.get(provider,'name')
        file = DavFile(id='root',provider=provider,parent_id='0',kind=0,name=name,size='0',create_time=formatted_time,download_url=None)
        files.append(file)
    return files


@app.get("/models",summary='所有模型',description='获取所有模型')
async def models():
    models_dir = os.path.join(os.getcwd(), 'models')
    module_files = glob.glob(os.path.join(models_dir, "*.py"))
    module_names = [os.path.basename(f)[:-3] for f in module_files if not f.endswith("__init__.py")]
    models = []
    for module_name in module_names:
        module_path = f'models.{module_name}'
        module = __import__(module_path)
        class_ = getattr(module, module_name)
        class_docstring = class_.__doc__.strip()
        model_info = {}
        model_info['name'] = module_name
        model_info['comment'] = class_docstring
        model_info['params'] = []
        init_docstring = class_.__init__.__doc__
        init_param_info = {}
        if init_docstring:
            init_doc_lines = init_docstring.strip().split("\n")
            for line in init_doc_lines:
                param_match = re.match(r":param\s+(\w+)\s*:\s*(.*)", line.strip())
                if param_match:
                    init_param_info['name'] = param_match.group(1)
                    init_param_info['comment'] = param_match.group(2)
                    model_info['params'].append(init_param_info)
        models.append(model_info)
    return models

@app.get("/dashboard", include_in_schema=False,response_class=HTMLResponse,summary='管理页面',description='管理页面')
async def dashboard(request: Request):
    return templates.TemplateResponse("index.html",{'request': request})

@app.get('/favicon.ico',include_in_schema=False)
async def favicon():
    file_name = "favicon.ico"
    file_path = os.path.join(app.root_path, "static")
    return FileResponse(path=f"{file_path}/{file_name}", headers={"Content-Disposition": "attachment; filename=" + file_name})


