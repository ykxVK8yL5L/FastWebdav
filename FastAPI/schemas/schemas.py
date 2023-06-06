from pydantic import BaseModel,Field,Extra
from typing import Optional

class DavFile(BaseModel):
    file_id: str = Field(title="文件ID",description="文件ID，如果是0和root需要注意",alias='id') 
    parent_id: str = Field(title="上级目录的ID",description="上级目录的ID,默认根目录为root需要特殊处理")
    provider: str = Field(title="模型实例的name",description="模型实例的name,通常不用管")
    kind: int = Field(title="文件类型",description="文件类型0为文件夹，1为文件") 
    name: str = Field(title="文件名称",description="文件名称") 
    size: str = Field(title="文件大小",description="文件大小，注意返回需要为字符串")  
    create_time:str = Field(title="文件创建时间",description="文件创建时间，需要格式化为年-月-日 时-分-秒的格式")
    sha1: Optional[str] = Field(title="文件sha1",description="文件sha1，可选")  
    download_url: Optional[str] = Field(title="文件下载链接",description="文件下载链接，有些可以在列表页算出来的就不需要请求了，可以添加?x-oss-expires=时间戳 来控制过期时间，如果rust的缓存时间先到以缓存时间为准")  #
   

class ListRequest(BaseModel):
    path_str:str = Field(title="请求的文件路径",description="求的文件路径，一般没用")  
    parent_file_id:str = Field(title="请求的目录ID",description="请求的文件上级目录ID") 