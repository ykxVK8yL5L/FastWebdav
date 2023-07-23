# FastWebDAV
演示视频:https://youtu.be/NBlzV9wgQVU   
Docker主页:https://hub.docker.com/r/ykxvk8yl5l/fast-webdav   
用FastAPI为WebDAV提供数据。   
这个项目不是针对小白用户使用的，Alist用户不是目标用户，需要会Python   
之前的几个webdav项目都是通过原生rust实现，技术难度并不大，但是扩展性太差。后来想着用rust做个webdav然后数据通过api提供统一格式就可以自由的扩展了，所以才有了这个项目。
说到数据Python是获取数据的最佳选择了。才用FastAPI负责后端数据提供。具体接口信息可以看下:8000/docs里面的接口文档。   
# 使用
## 加入解密功能，还不完善，稍后再细说 需要在configs目录新建 encrypt_dirs.ini内容为: 
```
[path] #加密路径，目前以starts_with进行判断
password=123456  #加密密码
```
目前除了Docker还没有其它渠道使用该项目【 https://hub.docker.com/r/ykxvk8yl5l/fast-webdav 】  映射本地目录到/root/configs    
如果本地部署fastapi建议使用以下命令启动
```
 uvicorn main:app --reload --reload-include '*.ini'
```
第一次访问后会生成providers.ini这个是数据提供的入口，里面的配置并不是目录的配置，以下是示例文件
```
[tmplink]
provider = TmpLink(provider="tmplink",token='XXXXXXXXXXXXXXXXXXXXXXXX')
name = tmplink
[stariver]
provider = Stariver(provider="stariver",token='XXXXXXXXXXXXXXXXXXXXXXXXXX')
name = stariver
[pikpak]
provider = PikPak(provider="pikpak",username='XXXXXXXXXXX',password='XXXXXXXXXXXX')
name = pikpak
[meting]
provider = Meting(provider="meting",server='netease',playlist_id='60198')
name = meting
[neteasecloud]
provider = NeteaseCloudMusic(provider="neteasecloud",playlist_id='577991289',count=50)
name = neteasecloud
[gofile]
provider = GoFile(provider="gofile",token="XXXXXXXXXXXXXXXXXXXXX",contentId='XXXXXXXXXXXXXXXXXXXXX',websiteToken='7fd94ds12fds4')
name = gofile
[filebin]
provider = Filebin(provider="filebin",bin='XXXXXXXXXXXXXX')
name = filebin
[webdav]
provider = WebDAV(provider="webdav",url="http://xxxxxxxxxxxxxxxx",username='',password='')
name = webdav
```
上面的代码中定义了几个provider,里面是我内置的几个模型，其中meting是废的，由于接口没有提供size参数，固定了文件大小，再加上目前的接口基本无法提供完整数据仅供参考。   
配置字段[tmplink]这是唯一值相当于一个模型实例的名称   
provider这个的定义就是一个模型实例的代码，通过eval将这个字段的值转换成一个类的实例，里面的配置和参数可以在models目录下的类里自己处理   
name字段是要目录文件夹的显示名字【就当是文件夹名称吧】
## 新增新模型
大体框架已经搭好了，如果需要添加新模型，只需要在models目录内添加新的类即可。Webdav有很多操作目录还没实现，目前只做了列表和获取链接的接口.请求都是由webdav提供的固定格式。
```
def list_files(self, list_req:ListRequest): #这个是获取文件列表的方法返回DavFile列表即可
def get_url(self,dav_file:DavFile): #这个是获取文件链接的方法
```
下载链接有些可以在列表页面算出来就不用再实现了，里面有添加过期的方法有需要可以参考下   

其它操作稍后再完善吧，特别是上传，由于各个网站上传方法不一样，目前还没有想好要怎样固定数据格式。   
文件上传示例代码   
```
curl -T "文件名" "http://127.0.0.1:9867/"  --header 'OC-Checksum:sha1:文件名的sha1'
```
# 再次重申 Alist用户不是目标用户
